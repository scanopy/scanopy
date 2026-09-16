use super::walk::{MAX_DESYNC_RETRIES, MAX_TRANSPORT_RETRIES};
use super::*;
use std::collections::VecDeque;

const BASE: &str = "1.3.6.1.2.1.2.2.1.1";

fn ip() -> IpAddr {
    "192.0.2.1".parse().unwrap()
}

fn page(oids: &[&str]) -> Vec<Vec<u64>> {
    oids.iter()
        .map(|s| s.split('.').map(|p| p.parse().unwrap()).collect())
        .collect()
}

/// Serves canned pages of OIDs to `walk_subtree`. Once `pages` is drained it repeats
/// `repeat` forever, which is how a device that never advances its OID is modelled.
/// Only OIDs are stored — `Value` isn't `Clone`, and the walk's termination logic
/// only cares about OID progression — so each page mints fresh integer values.
struct MockTransport {
    pages: VecDeque<Vec<Vec<u64>>>,
    repeat: Option<Vec<Vec<u64>>>,
}

impl MockTransport {
    fn scripted(pages: &[Vec<Vec<u64>>]) -> Self {
        Self {
            pages: pages.iter().cloned().collect(),
            repeat: None,
        }
    }

    fn stalling(p: Vec<Vec<u64>>) -> Self {
        Self {
            pages: VecDeque::new(),
            repeat: Some(p),
        }
    }

    fn next_page(&mut self) -> Varbinds<'static> {
        self.pages
            .pop_front()
            .or_else(|| self.repeat.clone())
            .unwrap_or_default()
            .into_iter()
            .map(|o| (o, Value::Integer(1)))
            .collect()
    }
}

#[async_trait::async_trait]
impl SnmpWalkTransport for MockTransport {
    async fn walk_getbulk<'a>(&'a mut self, _from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
        Ok(WalkPage::Varbinds(self.next_page()))
    }

    async fn walk_getnext<'a>(&'a mut self, _from: &[u64]) -> Result<Varbinds<'a>> {
        Ok(self.next_page())
    }
}

/// One canned answer, so a test can put the shapes a live agent produces *between* good pages
/// rather than only at the end of them. [`MockTransport`] can only ever answer, which is why
/// the walk's behaviour after a bad answer went untested.
enum Answer {
    /// Varbinds, in wire order. An empty one is the zero-varbind page an agent sends with
    /// `tooBig` set — indistinguishable from a table that has ended, before this walk learned
    /// to ask again.
    Page(Vec<Vec<u64>>),
    /// The agent said the page it was asked for will not fit.
    TooBig,
    /// Nothing came back: a timeout, or a datagram lost on the way. Nothing beneath the walk
    /// retransmits, so this is one lost packet as the walk sees it.
    NoAnswer,
}

/// Serves scripted answers and records the page size each getbulk asked for.
struct FlakyTransport {
    answers: VecDeque<Answer>,
    /// What to answer once the script runs out. `None` ends the walk by leaving the subtree.
    tail: Option<Answer>,
    /// `max_repetitions` per getbulk, in order, so a test can assert the walk asked for less
    /// after being refused rather than repeating the request that failed.
    asked: Vec<u32>,
    /// Requests served, to catch a retry that turns into a spin.
    requests: usize,
}

impl FlakyTransport {
    fn new(answers: Vec<Answer>) -> Self {
        Self {
            answers: answers.into(),
            tail: None,
            asked: Vec::new(),
            requests: 0,
        }
    }

    /// Answers the script, then this for ever.
    fn then_always(mut self, tail: Answer) -> Self {
        self.tail = Some(tail);
        self
    }

    fn answer(&mut self) -> Result<Varbinds<'static>, Answer> {
        self.requests += 1;
        let next = self
            .answers
            .pop_front()
            .unwrap_or_else(|| match &self.tail {
                Some(Answer::TooBig) => Answer::TooBig,
                Some(Answer::NoAnswer) => Answer::NoAnswer,
                Some(Answer::Page(p)) => Answer::Page(p.clone()),
                // Out of the subtree and above everything served: the natural end of a column.
                None => Answer::Page(page(&["1.3.6.1.2.1.2.2.1.2.1"])),
            });
        match next {
            Answer::Page(oids) => Ok(oids
                .into_iter()
                .map(|o| (o, Value::Integer(1)))
                .collect::<Varbinds<'static>>()),
            other => Err(other),
        }
    }
}

#[async_trait::async_trait]
impl SnmpWalkTransport for FlakyTransport {
    async fn walk_getbulk<'a>(&'a mut self, _from: &[u64], max: u32) -> Result<WalkPage<'a>> {
        self.asked.push(max);
        match self.answer() {
            Ok(v) => Ok(WalkPage::Varbinds(v)),
            Err(Answer::TooBig) => Ok(WalkPage::Refused { error_status: 1 }),
            Err(_) => Err(anyhow::anyhow!("getbulk timed out")),
        }
    }

    async fn walk_getnext<'a>(&'a mut self, _from: &[u64]) -> Result<Varbinds<'a>> {
        match self.answer() {
            Ok(v) => Ok(v),
            Err(_) => Err(anyhow::anyhow!("getnext timed out")),
        }
    }
}

/// An agent that refuses the page size has named its own remedy, and the walk has to take it.
///
/// RFC 3416 lets an agent answer an over-large getbulk with `tooBig` and no varbinds rather
/// than with fewer varbinds, and `lldpRemSysDesc` — long free text, twenty rows to a page — is
/// the request that provokes it. The response carries error-status but a perfectly valid
/// request id and community, so it used to arrive as an empty page and end the column, which
/// takes the whole neighbour set with it (GH #685). net-snmp asks again for half as much,
/// which is why the reporter's `snmpbulkwalk` read a table this walk gave up on.
#[tokio::test]
async fn a_refused_page_size_is_asked_for_again_smaller() {
    let mut session = FlakyTransport::new(vec![
        Answer::TooBig,
        Answer::Page(page(&[
            "1.3.6.1.2.1.2.2.1.1.1",
            "1.3.6.1.2.1.2.2.1.1.2",
            "1.3.6.1.2.1.2.2.1.1.3",
        ])),
    ]);

    let mut seen = 0usize;
    let stop = walk_subtree(&mut session, ip(), BASE, |_suffix, _v| seen += 1)
        .await
        .unwrap();

    assert!(
        stop.is_complete(),
        "an agent that asked for a smaller page has not stopped answering, but the walk \
         reported {stop:?}"
    );
    assert_eq!(
        seen, 3,
        "every row behind the refused page must still be read"
    );
    assert!(
        session.asked[1] < session.asked[0],
        "the retry has to ask for less than the page that was refused, or it earns the same \
         refusal: asked {:?}",
        session.asked
    );
}

/// SNMP is UDP and nothing beneath the walk retransmits, so one dropped datagram used to end
/// a column outright — and a column ending marks its whole group non-authoritative, so the
/// server keeps what it holds and a first-ever scan records nothing at all. `snmpwalk`
/// defaults to five retransmissions; the daemon has to be at least as tolerant as the tool
/// operators use to prove the device is readable.
#[tokio::test]
async fn one_lost_datagram_does_not_end_a_column() {
    let mut session = FlakyTransport::new(vec![
        Answer::Page(page(&["1.3.6.1.2.1.2.2.1.1.1"])),
        Answer::NoAnswer,
        Answer::Page(page(&["1.3.6.1.2.1.2.2.1.1.2"])),
    ]);

    let mut seen = 0usize;
    let stop = walk_subtree(&mut session, ip(), BASE, |_suffix, _v| seen += 1)
        .await
        .unwrap();

    assert!(
        stop.is_complete(),
        "a walk that lost one datagram and then got its answer is not a short read, but it \
         reported {stop:?}"
    );
    assert_eq!(
        seen, 2,
        "the row behind the lost datagram must still be read"
    );
}

/// The empty-page shape of the same fault: an answer with no varbinds on it, from an agent
/// that skipped a beat rather than one that has finished. Re-asked like every other
/// wrong-shaped answer the walk handles, instead of being the one that is taken at its word.
#[tokio::test]
async fn an_answer_with_no_varbinds_is_asked_again() {
    let mut session = FlakyTransport::new(vec![
        Answer::Page(page(&["1.3.6.1.2.1.2.2.1.1.1"])),
        Answer::Page(Vec::new()),
        Answer::Page(page(&["1.3.6.1.2.1.2.2.1.1.2"])),
    ]);

    let mut seen = 0usize;
    let stop = walk_subtree(&mut session, ip(), BASE, |_suffix, _v| seen += 1)
        .await
        .unwrap();

    assert!(
        stop.is_complete(),
        "an empty page followed by the rest of the table is not a short read, but the walk \
         reported {stop:?}"
    );
    assert_eq!(seen, 2, "the row behind the empty page must still be read");
}

/// The other half of the retry: a device that has genuinely gone quiet must be reported as
/// such, promptly. Retrying for ever would turn one unreachable switch into the whole scan's
/// budget, which is the failure the desync retries are already bounded against.
#[tokio::test]
async fn a_device_that_stays_silent_is_still_reported_short() {
    let mut session = FlakyTransport::new(vec![Answer::Page(page(&["1.3.6.1.2.1.2.2.1.1.1"]))])
        .then_always(Answer::NoAnswer);

    let mut seen = 0usize;
    let stop = walk_subtree(&mut session, ip(), BASE, |_suffix, _v| seen += 1)
        .await
        .unwrap();

    assert!(
        !stop.is_complete(),
        "a device that never answered again cannot have completed its table"
    );
    assert!(
        matches!(stop, WalkStop::Transport),
        "the reason has to survive to the warning — without it a timeout and a desynchronised \
         session are the same short column to an operator, but got {stop:?}"
    );
    assert_eq!(seen, 1, "the one page it did answer is still collected");
    assert!(
        session.requests < 10,
        "the walk must give up on a silent device rather than spin on it (made {} requests)",
        session.requests
    );
}

/// Serves getbulk and getnext from separate scripts, so a test can describe the one device
/// shape [`FlakyTransport`] cannot: an agent that answers one question and not the other.
///
/// GH #668. The reporter's switch timed out every getbulk on `lldpRemChassisId` and answered
/// `snmpwalk` — which is getnext — on the same column, and our walk never asked.
struct BulkMuteTransport {
    rows: Vec<Vec<u64>>,
    cursor: usize,
    /// Every getbulk asked for, to prove the walk stopped asking once it knew.
    bulk_requests: usize,
    /// Whether getnext answers too. `false` is a device that has gone silent outright.
    getnext_answers: bool,
    /// Survives across walks, as a real session does — which is what makes the second column
    /// able to know what the first one learned.
    getbulk_unusable: bool,
}

impl BulkMuteTransport {
    fn new(rows: &[&str]) -> Self {
        Self {
            rows: page(rows),
            cursor: 0,
            bulk_requests: 0,
            getnext_answers: true,
            getbulk_unusable: false,
        }
    }

    /// Nothing answers at all — the other half of the fallback, which still has to terminate.
    fn mute() -> Self {
        Self {
            getnext_answers: false,
            ..Self::new(&[])
        }
    }
}

#[async_trait::async_trait]
impl SnmpWalkTransport for BulkMuteTransport {
    fn getbulk_unusable(&self) -> bool {
        self.getbulk_unusable
    }

    fn note_getbulk_unusable(&mut self) {
        self.getbulk_unusable = true;
    }

    async fn walk_getbulk<'a>(&'a mut self, _from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
        self.bulk_requests += 1;
        Err(anyhow::anyhow!("getbulk timed out"))
    }

    async fn walk_getnext<'a>(&'a mut self, _from: &[u64]) -> Result<Varbinds<'a>> {
        if !self.getnext_answers {
            return Err(anyhow::anyhow!("getnext timed out"));
        }
        // One varbind per request, which is all a getnext ever returns, then out of the
        // subtree to end the column the way a real agent does.
        let row = self.rows.get(self.cursor).cloned();
        self.cursor += 1;
        Ok(vec![(
            row.unwrap_or_else(|| oids::oid_parts("1.3.6.1.2.1.2.2.1.2.1")),
            Value::Integer(1),
        )])
    }
}

/// GH #668, and the whole of it: a column the device will only serve one varbind at a time is
/// read in full, rather than abandoned with nothing.
///
/// `shrink_page` halves and only abandons getbulk once the page is down to 1 — five halvings
/// from `BULK_MAX_REPETITIONS`, against the two `MAX_TRANSPORT_RETRIES` allows. So the walk
/// spent its whole budget asking the same unanswerable question in three sizes. On the
/// reporter's switch3 that lost `lldpRemChassisId` outright, and with it all thirteen
/// neighbours, because a neighbour with no chassis id is discarded.
#[tokio::test]
async fn a_column_only_getnext_can_serve_is_read_to_its_end() {
    let mut session = BulkMuteTransport::new(&[
        "1.3.6.1.2.1.2.2.1.1.1",
        "1.3.6.1.2.1.2.2.1.1.2",
        "1.3.6.1.2.1.2.2.1.1.3",
    ]);

    let mut seen = 0usize;
    let stop = walk_subtree(&mut session, ip(), BASE, |_suffix, _v| seen += 1)
        .await
        .unwrap();

    assert!(
        stop.is_complete(),
        "the device answered every getnext it was asked, so the column is finished — \
         reporting {stop:?} marks the whole group non-authoritative"
    );
    assert_eq!(seen, 3, "every row the device would serve has to be read");
}

/// The budget the fallback is reached on is the same one it was reached with. A device that
/// answers nothing must still end promptly, still report short, and still name the reason.
#[tokio::test]
async fn a_device_answering_neither_request_still_terminates_and_reports_why() {
    let mut session = BulkMuteTransport::mute();

    let mut seen = 0usize;
    let stop = walk_subtree(&mut session, ip(), BASE, |_suffix, _v| seen += 1)
        .await
        .unwrap();

    assert_eq!(seen, 0);
    assert!(!stop.is_complete());
    assert!(
        matches!(stop, WalkStop::Transport),
        "an unreachable agent is a transport verdict, not a table that ended: {stop:?}"
    );
    assert!(
        session.bulk_requests <= MAX_TRANSPORT_RETRIES as usize + 1,
        "falling back must not buy the walk extra attempts against a silent device (made {} \
         getbulk requests)",
        session.bulk_requests
    );
}

/// Once a session has proved it cannot serve getbulk, the next column does not pay to find
/// out again.
///
/// switch3 spent fifteen seconds learning this on `lldpRemChassisId` and then fifteen more on
/// `lldpRemManAddrIfSubtype` (`switch-3-4-7_log.txt:45-48`). The knowledge belongs to the
/// session, which lives as long as the host, so a device costs it once.
#[tokio::test]
async fn a_session_that_fell_back_does_not_re_try_getbulk_on_the_next_column() {
    let mut session = BulkMuteTransport::new(&["1.3.6.1.2.1.2.2.1.1.1"]);

    walk_subtree(&mut session, ip(), BASE, |_suffix, _v| {})
        .await
        .unwrap();
    let after_first = session.bulk_requests;
    assert!(
        after_first > 0,
        "the first column has to actually try getbulk, or this proves nothing"
    );

    session.cursor = 0;
    walk_subtree(&mut session, ip(), BASE, |_suffix, _v| {})
        .await
        .unwrap();

    assert_eq!(
        session.bulk_requests, after_first,
        "the second column asked getbulk again on a session already known not to answer it"
    );
}

/// A device that keeps answering with the same in-subtree OID must not walk to the
/// entry cap (which on a live host burns the whole integration budget) — the
/// strict-advance guard has to cut it short and report a partial walk.
#[tokio::test]
async fn walk_terminates_when_oid_does_not_advance() {
    let mut session = MockTransport::stalling(page(&["1.3.6.1.2.1.2.2.1.1.5"]));

    let mut seen = 0usize;
    let complete = walk_subtree(&mut session, ip(), BASE, |_suffix, _v| seen += 1)
        .await
        .unwrap()
        .is_complete();

    assert!(!complete, "a non-advancing walk must report as partial");
    assert!(
        seen < MAX_WALK_ENTRIES,
        "guard must stop the walk, not run to the cap (saw {seen} entries)"
    );
}

/// The guard must not fire on a normal multi-page walk: each page's tail OID
/// strictly exceeds the OID it was requested from.
#[tokio::test]
async fn walk_completes_across_advancing_pages() {
    let mut session = MockTransport::scripted(&[
        page(&["1.3.6.1.2.1.2.2.1.1.1", "1.3.6.1.2.1.2.2.1.1.2"]),
        page(&["1.3.6.1.2.1.2.2.1.1.3", "1.3.6.1.2.1.2.2.1.1.4"]),
        // Next column — outside the base subtree, so the walk ends naturally.
        page(&["1.3.6.1.2.1.2.2.1.2.1"]),
    ]);

    let mut suffixes = Vec::new();
    let complete = walk_subtree(&mut session, ip(), BASE, |suffix, _v| {
        suffixes.push(suffix.to_vec())
    })
    .await
    .unwrap()
    .is_complete();

    assert!(
        complete,
        "a walk that reaches the end of the subtree is complete"
    );
    assert_eq!(suffixes, vec![vec![1], vec![2], vec![3], vec![4]]);
}

/// GH #710's Hikvision answers a getbulk it will not serve with an error status and the
/// request echoed back, so the one varbind on the page is the column base. Read as a row, that
/// is an answer to some other question, and the walk spent its desync budget re-asking until
/// it truncated every table with nothing. No error status makes an echo a row.
#[test]
fn an_error_status_response_is_never_read_as_rows() {
    let base = oids::oid_parts(BASE);
    for error_status in 1..=18 {
        let page = WalkPage::from_bulk_response(error_status, vec![(base.clone(), Value::Null)]);
        assert!(
            matches!(page, WalkPage::Refused { .. }),
            "a getbulk answered with error-status {error_status} was read as rows"
        );

        let read = response_varbinds(&base, error_status, vec![(base.clone(), Value::Null)]);
        assert!(
            !matches!(read.as_deref(), Ok([(_, Value::Null)])),
            "a getnext answered with error-status {error_status} handed its echo back as a row"
        );
    }
}

/// The staleness guard still stands. The same echoed column base *without* an error status
/// is a valid response carrying another request's OID, which is what a forking `pass` handler
/// under load sends, and the walk re-asks it and then reports the column desynchronised.
#[tokio::test]
async fn the_echoed_base_without_an_error_status_is_still_a_stale_answer() {
    let mut session = FlakyTransport::new(Vec::new()).then_always(Answer::Page(page(&[BASE])));

    let mut seen = 0usize;
    let stop = walk_subtree(&mut session, ip(), BASE, |_suffix, _v| seen += 1)
        .await
        .unwrap();

    assert!(
        matches!(stop, WalkStop::StaleResponse),
        "a wrong OID with no error status is a desync, got {stop:?}"
    );
    assert_eq!(seen, 0);
    assert_eq!(
        session.requests,
        1 + MAX_DESYNC_RETRIES as usize,
        "the guard re-asks exactly its budget before giving up"
    );
}

/// Getnext has no smaller question to fall back to. An agent that keeps refusing it ends the
/// column with the error status on record, not as a stale or non-advancing answer, and within
/// the retry budget.
#[tokio::test]
async fn a_getnext_the_agent_keeps_refusing_ends_as_an_error_status() {
    struct RefusesGetnext {
        requests: usize,
    }

    #[async_trait::async_trait]
    impl SnmpWalkTransport for RefusesGetnext {
        async fn walk_getbulk<'a>(&'a mut self, _from: &[u64], _max: u32) -> Result<WalkPage<'a>> {
            Ok(WalkPage::BulkUnsupported)
        }

        async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
            self.requests += 1;
            response_varbinds(from, 5, vec![(from.to_vec(), Value::Null)])
        }
    }

    let mut session = RefusesGetnext { requests: 0 };
    let stop = walk_subtree(&mut session, ip(), BASE, |_suffix, _v| {})
        .await
        .unwrap();

    assert!(
        matches!(stop, WalkStop::ErrorStatus),
        "a refused getnext is neither stale nor non-advancing, got {stop:?}"
    );
    assert!(
        session.requests <= 1 + MAX_TRANSPORT_RETRIES as usize,
        "the walk must give up on a refusing agent within budget (made {} requests)",
        session.requests
    );
}
