//! A simulated device as an SNMP transport.
//!
//! This emulates *snmpd driving a `pass` handler*, not a sorted map. That distinction is the whole
//! reason a device test proves anything:
//!
//! - A GETBULK of `n` is `n` successive handler GETNEXTs, each starting from the previous answer,
//!   because that is how snmpd assembles a bulk response over a `pass` script that answers one
//!   varbind at a time. A model that returned a slice of a sorted table would make every page
//!   perfect and could never reproduce a walk that ends early.
//! - A request is routed to exactly one registration — the `pass` line whose subtree covers it —
//!   and that registration's answer is bounded to its own subtree. Falling off the end moves to
//!   the next registration, which is what makes a device serving nothing at a subtree report the
//!   next one up rather than inventing an answer.
//! - The three handlers (`Normal`, `Positional`, `Stuck`) are the three shell scripts in
//!   `tools/snmp/lxc/setup.sh`, and two of them are meant to misbehave.
//! - Refusing GETBULK is not a handler. It happens in front of the agent, so it is a property of
//!   the device keyed by subtree — see [`SimAgent::refusing_getbulk`].
//! - Registrations are disjoint, and a request goes to the first one covering it. net-snmp serves
//!   only the broadest of a set of overlapping `pass` lines and ignores the rest, so serving one
//!   sub-table of a MIB differently means carving the MIB up, not nesting inside it.

use anyhow::Result;

use super::wire::{DataFile, Row};
use crate::daemon::discovery::integration::snmp::queries::{
    SNMP_ERR_NO_SUCH_NAME, SnmpWalkTransport, Varbinds, WalkPage, response_varbinds,
};

/// A device that answers a GETBULK above a repetition count with an error status instead of rows.
///
/// The GH #710 Hikvision shape, and like the GH #668 refusal it sits in front of the agent rather
/// than in a handler: a `pass` script sees one GETNEXT at a time and never the repetition count,
/// and net-snmp's own `maxGetbulkRepeats` returns fewer varbinds rather than an error. Mirrors
/// `snmp-bulk-refuser.py --reject-above`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BulkRejection {
    pub above_repetitions: u32,
    pub error_status: u32,
}

/// The varbinds an agent sends back with an error status: the request's own, value NULL, as
/// RFC 3416 has it.
fn echo(from: &[u64]) -> Varbinds<'static> {
    vec![(from.to_vec(), snmp2::Value::Null)]
}

/// Which `pass` handler serves a registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Handler {
    /// `snmp-pass-handler.sh` — answers GETNEXT with the first line numerically greater than the
    /// request. Every device but two.
    #[default]
    Normal,
    /// `snmp-pass-handler-unsorted.sh` — answers with the line that physically *follows* the one
    /// asked for, in file order. Firmware that stores a table unsorted and iterates it
    /// positionally does this, and it is why `snmpwalk` stops at "OID not increasing" where
    /// `snmpbulkwalk -Cc` reads the table in full (GH #674). The normal handler cannot reproduce
    /// it: it can only ever produce an ascending sequence, so a shuffled file would simply end the
    /// walk early.
    Positional,
    /// `snmp-pass-handler-stuck.sh` — answers every GETNEXT with the first line, whatever was
    /// asked. The non-advancing agent the walk's retry-then-stop guard was written for.
    Stuck,
}

impl Handler {
    /// One handler invocation: what the script prints for `GETNEXT from`, or nothing.
    fn getnext<'a>(self, file: &'a ServedFile, from: &[u64]) -> Option<&'a Row> {
        let rows = &file.rows;
        match self {
            Self::Normal => rows.iter().find(|row| row.oid.as_slice() > from),
            Self::Stuck => rows.first(),
            Self::Positional => {
                // The line after the requested one, in file order. A request naming no line of its
                // own — a bare column or table prefix — is answered with the first line under it,
                // again in file order, which is where the shuffle first shows.
                if let Some(pos) = rows.iter().position(|row| row.oid.as_slice() == from) {
                    return rows.get(pos + 1);
                }
                rows.iter()
                    .find(|row| row.oid.len() > from.len() && row.oid.starts_with(from))
                    .or_else(|| rows.iter().find(|row| row.oid.as_slice() > from))
            }
        }
    }
}

/// A `pass` line: one subtree, served from one file by one handler.
#[derive(Debug, Clone)]
pub struct Registration {
    pub subtree: Vec<u64>,
    pub file: usize,
    pub handler: Handler,
}

struct ServedFile {
    rows: Vec<Row>,
}

/// A device answering SNMP the way its deployed agent does.
pub struct SimAgent {
    files: Vec<ServedFile>,
    /// Ascending by subtree, which is the order snmpd walks its registrations in.
    registrations: Vec<Registration>,
    /// The agent has no getbulk, because SNMPv1 has none.
    ///
    /// Not a knob: it follows from the device's credential. A v1 agent answers a getbulk with an
    /// error, and the walk falls back to getnext — one varbind per round trip, for every column of
    /// every table. That fallback is most of what GH #557 is about, and a transport that always
    /// answered getbulk could not exercise it.
    bulk_unsupported: bool,
    /// Subtrees this device will not serve a GETBULK for, in front of the agent rather than in it.
    ///
    /// Mirrors `snmp-bulk-refuser.py`, which takes exactly this list of prefixes and drops any
    /// GETBULK whose first varbind falls under one. Keyed by subtree rather than by handler
    /// because that is what the shim knows: it reads the PDU type and the first OID off the wire
    /// and has no idea which `pass` line would have served it.
    refuses_getbulk: Vec<Vec<u64>>,
    /// Subtrees this device answers nothing at all for, getbulk or getnext.
    ///
    /// The same shim with its `--silence` list: a request whose first varbind falls under one of
    /// these is dropped whatever its PDU type, so every retry the walk has on that column times
    /// out and the column ends having read nothing. The rest of the table answers normally, which
    /// is what makes the walk stop part way rather than never start.
    silent_on: Vec<Vec<u64>>,
    /// A GETBULK above this size is answered with an error status and the request echoed back.
    rejects_getbulk: Option<BulkRejection>,
    /// Set once a walk has fallen back to getnext, and read by every walk after it.
    ///
    /// One `SimAgent` serves a whole collection, exactly as one session serves one host, so this
    /// is what lets a device test observe the per-host half of the GH #668 fix — a device costing
    /// the scan one column's worth of timeouts rather than one per column — and not only the
    /// per-column half.
    getbulk_unusable: bool,
}

impl SimAgent {
    pub fn new(files: &[DataFile], registrations: Vec<Registration>) -> Self {
        let files = files
            .iter()
            .map(|file| ServedFile {
                // `rows()` applies the file's ordering, so a `Positional` file keeps the order it
                // was written in and every other file is sorted — exactly what lands on the VM.
                rows: file.rows(),
            })
            .collect();
        let mut registrations = registrations;
        registrations.sort_by(|a, b| a.subtree.cmp(&b.subtree));
        Self {
            files,
            registrations,
            bulk_unsupported: false,
            refuses_getbulk: Vec::new(),
            silent_on: Vec::new(),
            rejects_getbulk: None,
            getbulk_unusable: false,
        }
    }

    /// An agent behind a shim that drops every request for these subtrees.
    pub fn silent_on(mut self, subtrees: Vec<Vec<u64>>) -> Self {
        self.silent_on = subtrees;
        self
    }

    fn is_silent(&self, from: &[u64]) -> bool {
        self.silent_on.iter().any(|prefix| from.starts_with(prefix))
    }

    /// An agent behind a shim that answers an oversized GETBULK with an error status.
    pub fn rejecting_getbulk_above(mut self, rejection: Option<BulkRejection>) -> Self {
        self.rejects_getbulk = rejection;
        self
    }

    /// An agent behind a shim that drops GETBULK for these subtrees and forwards everything else.
    ///
    /// The GH #668 device: it will not answer a bulk page on its LLDP neighbour columns and will
    /// answer a getnext on the very same OIDs, so a walk that never falls back loses the column
    /// and one that does reads it in full.
    pub fn refusing_getbulk(mut self, subtrees: Vec<Vec<u64>>) -> Self {
        self.refuses_getbulk = subtrees;
        self
    }

    /// An agent with no getbulk, as SNMPv1 has none.
    pub fn without_getbulk(mut self) -> Self {
        self.bulk_unsupported = true;
        self
    }

    fn covers(registration: &Registration, oid: &[u64]) -> bool {
        oid.starts_with(&registration.subtree)
    }

    /// Route one GETNEXT the way snmpd does, then bound the answer to the registration that gave
    /// it. An answer outside its own subtree is not returned — net-snmp treats that registration
    /// as exhausted and moves on, which is what stops one `pass` file leaking rows into a walk of
    /// a subtree it does not serve.
    fn answer(&self, from: &[u64]) -> Option<&Row> {
        self.routed(from).map(|(_, row)| row)
    }

    /// The registration that answers `from`, alongside its row.
    ///
    /// Split out of [`Self::answer`] because a request's *cost* belongs to whichever `pass` line
    /// serves it, not to the device: one slow registration on a switch whose other tables answer
    /// instantly is the shape GH #668 arrived in, and a whole-agent flag could not express it.
    ///
    /// First match, and it may stay first match because registrations are disjoint —
    /// [`super::SimDevice::registrations`] builds them that way and
    /// `no_registration_is_nested_inside_another` holds every device to it. This deliberately does
    /// *not* prefer the longest match: net-snmp's `pass` serves only the broadest of a set of
    /// overlapping lines and silently ignores the rest, so a model that resolved nesting would be
    /// more capable than the agent it stands in for, and a fixture that worked here would go quiet
    /// on the VM. That is not hypothetical — it is what a nested LLDP registration did.
    fn routed(&self, from: &[u64]) -> Option<(&Registration, &Row)> {
        let start = self
            .registrations
            .iter()
            .position(|reg| Self::covers(reg, from))
            .or_else(|| {
                self.registrations
                    .iter()
                    .position(|reg| reg.subtree.as_slice() > from)
            })?;

        for registration in &self.registrations[start..] {
            let ask: &[u64] = if Self::covers(registration, from) {
                from
            } else {
                &registration.subtree
            };
            if let Some(row) = registration
                .handler
                .getnext(&self.files[registration.file], ask)
                .filter(|row| Self::covers(registration, &row.oid))
            {
                return Some((registration, row));
            }
        }
        None
    }

    /// A GETBULK page: `max_repetitions` chained GETNEXTs, as snmpd assembles one over a handler
    /// that answers a single varbind per invocation.
    ///
    /// The chaining is what lets a misbehaving handler show itself. A `Stuck` handler yields the
    /// same row `n` times; a `Positional` one yields a page whose last row can sort *below* its
    /// first, which is the exact moment a strictly-ascending walk gives up.
    fn page(&self, from: &[u64], max_repetitions: u32) -> Varbinds<'_> {
        let mut page = Vec::new();
        let mut cursor = from.to_vec();
        for _ in 0..max_repetitions.max(1) {
            match self.answer(&cursor) {
                Some(row) => {
                    page.push((row.oid.clone(), row.value.as_snmp()));
                    cursor = row.oid.clone();
                }
                None => break,
            }
        }
        // Past the last row a real agent says so rather than answering with nothing: an empty
        // response is abnormal and the walk rightly reads it as truncation.
        if page.is_empty() {
            return vec![(from.to_vec(), snmp2::Value::EndOfMibView)];
        }
        page
    }
}

#[async_trait::async_trait]
impl SnmpWalkTransport for SimAgent {
    fn getbulk_unusable(&self) -> bool {
        self.getbulk_unusable
    }

    fn note_getbulk_unusable(&mut self) {
        self.getbulk_unusable = true;
    }

    async fn walk_getbulk<'a>(
        &'a mut self,
        from: &[u64],
        max_repetitions: u32,
    ) -> Result<WalkPage<'a>> {
        if self.is_silent(from) {
            return Err(anyhow::anyhow!("getbulk timed out"));
        }
        if self.bulk_unsupported {
            return Ok(WalkPage::BulkUnsupported);
        }
        // A refused bulk is a datagram the shim drops, so the client sees silence and times out.
        // That is an `Err` — the shape a real timeout takes — and deliberately neither an empty
        // page nor `BulkUnsupported`, both of which the walk has always handled. `walk_getnext`
        // below is not guarded: the same OIDs answer perfectly well one varbind at a time, and
        // that asymmetry is the whole of GH #668.
        if self
            .refuses_getbulk
            .iter()
            .any(|prefix| from.starts_with(prefix))
        {
            return Err(anyhow::anyhow!("getbulk timed out"));
        }
        if let Some(rejection) = self.rejects_getbulk
            && max_repetitions > rejection.above_repetitions
        {
            return Ok(WalkPage::from_bulk_response(
                rejection.error_status,
                echo(from),
            ));
        }
        Ok(WalkPage::from_bulk_response(
            0,
            self.page(from, max_repetitions),
        ))
    }

    async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
        // A silenced column answers nothing by getnext either, which is what separates it from a
        // refused getbulk.
        if self.is_silent(from) {
            return Err(anyhow::anyhow!("getnext timed out"));
        }
        let page = self.page(from, 1);
        // SNMPv1 has no `endOfMibView`. net-snmp answers a v1 client that walks off the end of
        // its MIB view with `noSuchName` and the request echoed back (RFC 3584 §4.2.2.2.2), and
        // `bulk_unsupported` is exactly the v1 agent.
        if self.bulk_unsupported && matches!(page.as_slice(), [(_, snmp2::Value::EndOfMibView)]) {
            return response_varbinds(from, SNMP_ERR_NO_SUCH_NAME, echo(from));
        }
        response_varbinds(from, 0, page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::integration::snmp::oids::{arp, if_mib, oid_parts};
    use crate::daemon::discovery::integration::snmp::sim::wire::{Ordering, PassValue};

    fn arp_file(order: Ordering, indexes: &[u64]) -> DataFile {
        DataFile::new(
            "arp",
            order,
            indexes
                .iter()
                .map(|i| {
                    Row::at(
                        arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
                        &[1, 10, 20, 30, *i],
                        PassValue::Integer(1),
                    )
                })
                .collect(),
        )
    }

    fn agent(file: DataFile, base: &str, handler: Handler) -> SimAgent {
        SimAgent::new(
            &[file],
            vec![Registration {
                subtree: oid_parts(base),
                file: 0,
                handler,
            }],
        )
    }

    /// A page is chained GETNEXTs, so a well-behaved agent walks forward one row at a time.
    #[test]
    fn a_page_advances_one_row_per_repetition() {
        let agent = agent(
            arp_file(Ordering::Ascending, &[1, 2, 3, 4, 5]),
            arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
            Handler::Normal,
        );
        let page = agent.page(&oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX), 3);
        let last: Vec<u64> = page.iter().map(|(oid, _)| *oid.last().unwrap()).collect();
        assert_eq!(last, vec![1, 2, 3]);
    }

    /// GH #674, and the property the whole `.246` fixture rests on: served positionally, a page
    /// can end *lower* than it began. A sorted-map model cannot produce this, which is why the
    /// transport emulates the handler rather than the table.
    #[test]
    fn a_positional_agent_hands_back_a_page_that_ends_below_where_it_started() {
        // Evens then odds — the ordering the real fixture uses, so the second page ends lower
        // than the first.
        let agent = agent(
            arp_file(Ordering::Positional, &[2, 4, 6, 8, 1, 3, 5]),
            arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
            Handler::Positional,
        );
        let page = agent.page(&oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX), 6);
        let last: Vec<u64> = page.iter().map(|(oid, _)| *oid.last().unwrap()).collect();

        assert_eq!(last, vec![2, 4, 6, 8, 1, 3]);
        // The signature of the defect: somewhere in the page a row sorts *below* the one before
        // it. That is the moment a client insisting every step ascend gives up ("OID not
        // increasing") while `snmpbulkwalk -Cc` reads the table in full.
        assert!(
            last.windows(2).any(|pair| pair[1] < pair[0]),
            "a positional agent must hand back a descent: {last:?}"
        );
    }

    /// The non-advancing agent: every repetition is the same row, which is what the walk's
    /// retry-then-stop guard exists to survive.
    #[test]
    fn a_stuck_agent_answers_every_repetition_with_the_same_row() {
        let agent = agent(
            arp_file(Ordering::Ascending, &[1, 2, 3]),
            arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
            Handler::Stuck,
        );
        let page = agent.page(&oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX), 4);
        let last: Vec<u64> = page.iter().map(|(oid, _)| *oid.last().unwrap()).collect();
        assert_eq!(last, vec![1, 1, 1, 1]);
    }

    /// The broadest of two overlapping registrations answers, and the nested one is dead config.
    ///
    /// This is net-snmp, not a modelling choice. A `pass` line nested inside another is not served
    /// at all: an LLDP MIB registered at its root alongside `…1.3.7` and `…1.4` answered every
    /// request from the root line, so the sub-table handlers never ran and the rows moved into the
    /// sub-table's own file went unserved — the device reported `No Such Instance` for a table it
    /// held. Pinned here so the model cannot quietly become more capable than the agent, and
    /// enforced for real by `no_registration_is_nested_inside_another`, which stops any device
    /// producing this config in the first place.
    #[test]
    fn a_registration_nested_inside_another_is_never_reached() {
        let file = arp_file(Ordering::Ascending, &[1, 2, 3]);
        let inner = oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX);
        let outer = inner[..inner.len() - 1].to_vec();
        let agent = SimAgent::new(
            &[file],
            vec![
                Registration {
                    subtree: outer,
                    file: 0,
                    handler: Handler::Normal,
                },
                Registration {
                    subtree: inner.clone(),
                    file: 0,
                    handler: Handler::Stuck,
                },
            ],
        );

        // `Stuck` would answer every request with the first row. Advancing normally is the outer
        // `Normal` line taking the request, which is what the VM does.
        let page = agent.page(&inner, 3);
        let last: Vec<u64> = page.iter().map(|(oid, _)| *oid.last().unwrap()).collect();
        assert_eq!(last, vec![1, 2, 3]);
    }

    /// The asymmetry GH #668 turns on: the same OIDs refuse a getbulk and answer a getnext.
    ///
    /// Verified against the real shim before this was written — `snmpbulkwalk` through
    /// `snmp-bulk-refuser.py` times out on `1.0.8802.1.1.2.1.4.1.1.5` while `snmpwalk` returns all
    /// three rows of it, and a getbulk of a different subtree is untouched.
    #[tokio::test]
    async fn a_refused_subtree_answers_getnext_and_times_out_every_getbulk() {
        let base = oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX);
        let mut agent = agent(
            arp_file(Ordering::Ascending, &[1, 2, 3]),
            arp::entry::IP_NET_TO_MEDIA_IF_INDEX,
            Handler::Normal,
        )
        .refusing_getbulk(vec![base.clone()]);

        // Every page size the walk shrinks through, including the smallest it ever asks for. The
        // shim drops on the PDU type, so the size it asked for never mattered.
        for max_repetitions in [20, 10, 5, 2, 1] {
            assert!(
                agent.walk_getbulk(&base, max_repetitions).await.is_err(),
                "a refused subtree drops the datagram whatever the page size, and {max_repetitions} \
                 got an answer"
            );
        }

        let answered = agent.walk_getnext(&base).await.expect("getnext answers");
        assert_eq!(*answered[0].0.last().unwrap(), 1);
    }

    /// The refusal is scoped to its subtree. The reporter's switch served its ifTable and ARP
    /// table by bulk on the same scan that lost its LLDP neighbours, and a device that refused
    /// everything would be a different fixture — one the walk already handles.
    #[tokio::test]
    async fn a_refusal_does_not_spread_to_the_rest_of_the_device() {
        let refused = oid_parts(arp::entry::IP_NET_TO_MEDIA_IF_INDEX);
        let mut agent = SimAgent::new(
            &[
                arp_file(Ordering::Ascending, &[1, 2]),
                DataFile::new(
                    "iftable",
                    Ordering::Ascending,
                    vec![Row::at(
                        if_mib::columns::IF_INDEX,
                        &[1],
                        PassValue::Integer(1),
                    )],
                ),
            ],
            vec![
                Registration {
                    subtree: refused.clone(),
                    file: 0,
                    handler: Handler::Normal,
                },
                Registration {
                    subtree: oid_parts(if_mib::IF_TABLE),
                    file: 1,
                    handler: Handler::Normal,
                },
            ],
        )
        .refusing_getbulk(vec![refused.clone()]);

        assert!(agent.walk_getbulk(&refused, 10).await.is_err());
        assert!(matches!(
            agent.walk_getbulk(&oid_parts(if_mib::IF_TABLE), 10).await,
            Ok(WalkPage::Varbinds(_))
        ));
    }

    /// One file serving several subtrees must not leak rows across them: a walk of `ifTable` ends
    /// at the end of `ifTable`, even though the same file also holds the ifXTable rows that sort
    /// immediately above it.
    #[test]
    fn a_registration_does_not_answer_outside_its_own_subtree() {
        let file = DataFile::new(
            "iftable",
            Ordering::Ascending,
            vec![
                Row::at(if_mib::columns::IF_INDEX, &[1], PassValue::Integer(1)),
                Row::at(
                    if_mib::if_x_table::IF_NAME,
                    &[1],
                    PassValue::Str("Gi0/1".into()),
                ),
            ],
        );
        let agent = SimAgent::new(
            &[file],
            vec![Registration {
                subtree: oid_parts(if_mib::IF_TABLE),
                file: 0,
                handler: Handler::Normal,
            }],
        );

        // Walking off the end of ifTable finds nothing, rather than handing back the ifXTable row
        // that physically follows it in the same file.
        let page = agent.page(&oid_parts(if_mib::columns::IF_INDEX), 5);
        assert_eq!(page.len(), 1);
        assert_eq!(*page[0].0.last().unwrap(), 1);

        let past = agent.page(&oid_parts(if_mib::if_x_table::IF_NAME), 1);
        assert!(matches!(past[0].1, snmp2::Value::EndOfMibView));
    }
}
