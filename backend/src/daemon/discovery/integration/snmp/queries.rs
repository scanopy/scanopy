//! SNMP Query Functions
//!
//! Functions for querying SNMP data from devices.

use anyhow::Result;
use snmp2::{Oid, Value};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use tokio::time::timeout;
use tracing::{debug, trace, warn};

use crate::daemon::discovery::service::warnings::{
    ClaimSource, DeviceClaim, MalformedNeighbourReason, ShortfallReason,
};
use crate::server::lldp::canonical_mac;

use super::oids::{self, oid_to_vec};
use super::session::{MAX_WALK_ENTRIES, SNMP_TIMEOUT};
use super::types::{
    ArpEntry, BridgeFdbEntry, CdpNeighbor, DeviceInventory, IfTableEntry, IpAddrEntry,
    LldpLocalInfo, LldpLocalPort, LldpNeighbor, PortVlanMembership, SystemInfo, VlanInfo,
};
use super::values::{
    parse_lldp_mgmt_addr, parse_portlist_bitmap, qbridge_fdb_index_to_mac, value_to_i32,
    value_to_ip, value_to_mac, value_to_string, value_to_u16, value_to_u64, value_type_name,
};

/// Varbinds requested per getbulk round when walking a table subtree.
const BULK_MAX_REPETITIONS: u32 = 20;

/// A single `getbulk` round-trip's non-error outcome. Transport failures (timeouts,
/// session errors) are the `Err` arm of the returned `Result`; the legitimate non-error
/// signals are an agent that refuses getbulk, which the walk retries via getnext, and one
/// that says the page it was asked for will not fit, which the walk retries smaller.
/// Varbinds borrow the session's response buffer (`snmp2::Value<'a>` holds `&'a [u8]`
/// for octet strings), so a page is only valid while the session stays borrowed.
pub type Varbinds<'a> = Vec<(Vec<u64>, Value<'a>)>;

/// SNMP `tooBig(1)` — the response to this request would exceed what the agent can send.
///
/// RFC 3416 lets an agent answer an over-large getbulk this way instead of returning fewer
/// varbinds, and it does so with an *empty* varbind list. `Pdu::validate` checks message type,
/// request id and community and ignores `error-status` entirely, so without this the response
/// arrived as a zero-varbind page and ended the column as [`WalkStop::EmptyResponse`] — a
/// device answering "ask me for less" reported as one that had gone silent.
const SNMP_ERR_TOO_BIG: u32 = 1;

pub enum WalkPage<'a> {
    /// Decoded varbinds in wire order, OIDs as sub-id vectors.
    Varbinds(Varbinds<'a>),
    /// Agent rejected getbulk (e.g. SNMPv1) — retry from the same OID with getnext.
    BulkUnsupported,
    /// Agent answered `tooBig` — retry from the same OID with fewer repetitions.
    TooBig,
}

/// The SNMP operations the query layer needs. Abstracting them keeps the walk loop
/// transport-agnostic so its termination logic is unit-testable without a live UDP
/// socket. Two implementors only: `Box<AsyncSession>` in production (below) and a
/// canned-page mock under `#[cfg(test)]`.
///
/// The `Send` supertrait is load-bearing beyond spawning: it is what lets [`Self::get_scalar`]
/// carry a default body, because `async_trait` adds an implicit `Self: Send` bound to any
/// provided `&mut self` method. Removing it would make every test fake prove `Send` by hand.
#[async_trait::async_trait]
pub trait SnmpWalkTransport: Send {
    async fn walk_getbulk<'a>(
        &'a mut self,
        from: &[u64],
        max_repetitions: u32,
    ) -> Result<WalkPage<'a>>;
    async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>>;

    /// Read one scalar instance, e.g. `sysName.0`.
    ///
    /// `Ok(None)` is "the agent has nothing at that OID" — a `noSuchObject`, a `noSuchInstance`,
    /// an empty varbind list, or a next-OID that is not the one asked for. `Err` is the transport.
    /// The three are collapsed because every caller already treats them identically; if a scalar
    /// ever needs "unimplemented" told apart from "absent", this should return
    /// `Some(Value::NoSuchObject)` rather than grow a third arm.
    ///
    /// The default body exists so the whole fake-transport suite gains scalar support without
    /// edits: `query_system_info` and `query_lldp_local` took `&mut Box<AsyncSession>` concretely
    /// and so were the only two SNMP queries with no test and no way to reach them from a fake.
    /// Production overrides it with a real GET.
    async fn get_scalar<'a>(&'a mut self, oid: &[u64]) -> Result<Option<Value<'a>>> {
        // GETNEXT from the OID with its last sub-id removed is a GET expressed in the operations
        // every transport already has: `sysName` is the immediate lexicographic predecessor of
        // `sysName.0` — nothing can sort strictly between `P` and `P.0` — so an agent's first
        // varbind for it is that instance when it exists and something else when it does not.
        // Requiring an exact OID match is what stops "something else" (the next column, the next
        // MIB object) being read as the scalar; that mis-read is silent and lands one device's
        // identity on another.
        let Some((_, parent)) = oid.split_last() else {
            return Ok(None);
        };
        Ok(self
            .walk_getnext(parent)
            .await?
            .into_iter()
            .find(|(resp, _)| resp.as_slice() == oid)
            .map(|(_, value)| value)
            .filter(|value| {
                !matches!(
                    value,
                    Value::NoSuchObject | Value::NoSuchInstance | Value::EndOfMibView
                )
            }))
    }
}

#[async_trait::async_trait]
impl SnmpWalkTransport for Box<snmp2::AsyncSession> {
    async fn walk_getbulk<'a>(
        &'a mut self,
        from: &[u64],
        max_repetitions: u32,
    ) -> Result<WalkPage<'a>> {
        let oid = Oid::from(from).map_err(|_| anyhow::anyhow!("invalid walk OID"))?;
        match timeout(SNMP_TIMEOUT, self.getbulk(&[&oid], 0, max_repetitions)).await {
            Ok(Ok(pdu)) if pdu.error_status == SNMP_ERR_TOO_BIG => Ok(WalkPage::TooBig),
            Ok(Ok(pdu)) => Ok(WalkPage::Varbinds(
                pdu.varbinds.map(|(o, v)| (oid_to_vec(&o), v)).collect(),
            )),
            // A response that fails request-id or community validation is a session that has lost
            // sync with its own traffic, not an agent declining getbulk — treating it as "no bulk
            // support" produced a silently short table that still claimed to be complete. The
            // error type is preserved rather than formatted so `is_desync` can recognise it
            // without matching on message text.
            Ok(Err(e @ (snmp2::Error::RequestIdMismatch | snmp2::Error::CommunityMismatch))) => {
                Err(anyhow::Error::new(e).context("SNMP session desynchronized"))
            }
            Ok(Err(_)) => Ok(WalkPage::BulkUnsupported),
            Err(_) => Err(anyhow::anyhow!("getbulk timed out")),
        }
    }

    async fn walk_getnext<'a>(&'a mut self, from: &[u64]) -> Result<Varbinds<'a>> {
        let oid = Oid::from(from).map_err(|_| anyhow::anyhow!("invalid walk OID"))?;
        match timeout(SNMP_TIMEOUT, self.getnext(&oid)).await {
            Ok(Ok(pdu)) => Ok(pdu.varbinds.map(|(o, v)| (oid_to_vec(&o), v)).collect()),
            Ok(Err(e)) => Err(anyhow::Error::new(e).context("getnext failed")),
            Err(_) => Err(anyhow::anyhow!("getnext timed out")),
        }
    }

    /// A real GET rather than the trait's GETNEXT emulation: it is what the device is asked in
    /// production, and on an agent whose scalar is absent it says so instead of handing back
    /// whatever object happens to sort next. The two filters match the default body exactly, so
    /// a fake and a live session cannot disagree about what "nothing there" looks like.
    async fn get_scalar<'a>(&'a mut self, oid: &[u64]) -> Result<Option<Value<'a>>> {
        let requested = Oid::from(oid).map_err(|_| anyhow::anyhow!("invalid scalar OID"))?;
        match timeout(SNMP_TIMEOUT, self.get(&requested)).await {
            Ok(Ok(mut response)) => Ok(response
                .varbinds
                .next()
                .filter(|(resp, _)| oid_to_vec(resp) == oid)
                .map(|(_, value)| value)
                .filter(|value| {
                    !matches!(
                        value,
                        Value::NoSuchObject | Value::NoSuchInstance | Value::EndOfMibView
                    )
                })),
            Ok(Err(e)) => Err(anyhow::Error::new(e).context("get failed")),
            Err(_) => Err(anyhow::anyhow!("get timed out")),
        }
    }
}

/// Walk the OID subtree rooted at `base_oid_str`, invoking `on_entry(suffix, value)`
/// for every varbind under it, where `suffix` is the OID sub-ids after the base.
///
/// Uses SNMP `getbulk` for throughput (one round returns up to `BULK_MAX_REPETITIONS`
/// varbinds instead of one per round-trip) and transparently falls back to `getnext`
/// if the agent rejects getbulk (e.g. SNMPv1).
///
/// How many times one walk step will re-issue its request after reading someone else's answer.
///
/// Small deliberately. A retry costs one round trip and only happens on a genuine desync, but a
/// device that is *persistently* answering out of step should be reported as truncated rather
/// than have the scan spin on it.
const MAX_DESYNC_RETRIES: u8 = 2;

/// How many times one walk step will re-issue its request after getting no usable answer at all —
/// a timeout, a session error, or a page with no varbinds on it.
///
/// SNMP runs over UDP and nothing beneath us retransmits: `AsyncSession::send_and_recv` is one
/// `send` and one `recv`, bounded only by [`SNMP_TIMEOUT`]. `snmpwalk` defaults to `-r 5 -t 1`, so
/// until this existed the daemon was strictly less tolerant than the command line operators use to
/// prove a device is readable — a single dropped datagram in any one of the seven LLDP columns
/// ended that column, which marks the whole neighbour set non-authoritative and leaves the switch
/// looking as though it has no LLDP at all (GH #685).
///
/// Counted separately from [`MAX_DESYNC_RETRIES`] so a device suffering both faults cannot spend
/// one budget on the other. Kept as small as the desync budget for the same reason: a device that
/// has genuinely stopped answering should be reported, not spun on.
const MAX_TRANSPORT_RETRIES: u8 = 2;

/// Ask the agent for half as much next time, and give up on getbulk entirely once even a
/// single-repetition page has not worked.
///
/// Halving rather than dropping straight to getnext because the difference is a whole table's
/// worth of round trips: a 10000-entry FDB read one varbind at a time is the shape the walk
/// timeout was raised for. This is what net-snmp does with `tooBig`, and it is why the reporter's
/// `snmpbulkwalk` read a table our walk gave up on.
fn shrink_page(max_reps: &mut u32, use_bulk: &mut bool) {
    if *max_reps > 1 {
        *max_reps = (*max_reps / 2).max(1);
    } else {
        *use_bulk = false;
    }
}

/// Whether this error means the session read an answer to a question nobody is waiting for.
///
/// The daemon abandons SNMP requests constantly — 5s per query, 60s per walk — and keeps using
/// the session afterwards. `drain_stale` clears the socket before each send, but a response still
/// in flight from an abandoned request lands *after* that drain and is read by the next `recv`,
/// where it fails validation. That is a transient belonging to the previous request, not a
/// verdict on this one, and ending the walk on it turned one slow answer into a truncated table
/// — visible in a customer log as `GET timeout` followed immediately by `RequestIdMismatch`.
///
/// Re-issuing is safe precisely because the failed read *consumed* the stale datagram, so the
/// retry cannot be served the same one again.
fn is_desync(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<snmp2::Error>(),
            Some(snmp2::Error::RequestIdMismatch | snmp2::Error::CommunityMismatch)
        )
    })
}

/// Returns why the walk stopped. [`WalkStop::is_complete`] is true when the subtree was walked
/// to its natural end (or the agent said it has no such OID) and false when it was cut short by
/// `MAX_WALK_ENTRIES`, a session error, a timeout, a non-advancing OID, or an abnormal empty
/// response — callers that prune against a full table (see `walk_if_table`, GH #649) must treat
/// an incomplete walk as partial. The reason itself is returned rather than collapsed to a bool
/// because "the agent has no such OID" and "the table is implemented and empty" are different
/// answers with different consequences; see [`WalkStop::is_unsupported`].
async fn walk_subtree<T, F>(
    session: &mut T,
    ip: IpAddr,
    base_oid_str: &str,
    mut on_entry: F,
) -> Result<WalkStop>
where
    T: SnmpWalkTransport,
    F: FnMut(&[u64], &Value),
{
    let base_parts: Vec<u64> = oids::oid_parts(base_oid_str);

    let mut current_parts = base_parts.clone();
    let mut count = 0usize;
    let mut use_bulk = true;
    let mut stop = WalkStop::EndOfSubtree;
    let mut stop_detail: Option<String> = None;
    let mut desync_retries = 0u8;
    let mut transport_retries = 0u8;
    // Repetitions asked for per getbulk round. Walk-local rather than the constant because it only
    // ever shrinks: an agent that could not fit 20 varbinds will not fit them on the next page
    // either, so re-escalating would just re-earn the same failure.
    let mut max_reps = BULK_MAX_REPETITIONS;
    // Every in-subtree OID already handed to `on_entry`. This is what tells the two devices that
    // used to look identical apart: an OID below where we asked from is the GH #674 firmware bug
    // when it names a row we have not seen, and an agent going in circles when it does not.
    // Bounded by `MAX_WALK_ENTRIES` because nothing is inserted without also counting.
    let mut seen: HashSet<Vec<u64>> = HashSet::new();
    // The highest in-subtree OID accepted so far. The staleness test below has to compare against
    // this rather than the cursor: the cursor now follows the agent's own order and may sit below
    // rows already collected, and comparing against it would let a genuinely stale response pass.
    let mut high_water = base_parts.clone();

    'walk: loop {
        if count >= MAX_WALK_ENTRIES {
            stop = WalkStop::EntryCap;
            break;
        }

        let varbinds = if use_bulk {
            match session.walk_getbulk(&current_parts, max_reps).await {
                Ok(WalkPage::Varbinds(v)) => v,
                Ok(WalkPage::BulkUnsupported) => {
                    // Agent rejected getbulk (e.g. v1) — retry from the same OID with
                    // getnext and stay on getnext for the rest of this walk.
                    use_bulk = false;
                    continue 'walk;
                }
                Ok(WalkPage::TooBig) => {
                    // The agent named its own remedy, so this is not a retry against a budget —
                    // halving terminates on its own (20 → 10 → 5 → 2 → 1 → getnext) and each
                    // round asks a strictly easier question than the one just refused.
                    shrink_page(&mut max_reps, &mut use_bulk);
                    debug!(
                        ip = %ip,
                        base = base_oid_str,
                        max_repetitions = max_reps,
                        getbulk = use_bulk,
                        "Agent refused the page size; asking for less"
                    );
                    continue 'walk;
                }
                Err(e) if is_desync(&e) && desync_retries < MAX_DESYNC_RETRIES => {
                    desync_retries += 1;
                    debug!(
                        ip = %ip,
                        base = base_oid_str,
                        attempt = desync_retries,
                        error = %e,
                        "Re-issuing after reading a stale answer"
                    );
                    continue 'walk;
                }
                Err(e) if transport_retries < MAX_TRANSPORT_RETRIES => {
                    transport_retries += 1;
                    // Shrink as well as retry. A timeout on this path is as likely to be the
                    // agent labouring over a large page as a lost datagram — the reporter's
                    // switch answered getbulk at roughly nine times the per-varbind cost of
                    // getnext — and a smaller page addresses both.
                    shrink_page(&mut max_reps, &mut use_bulk);
                    debug!(
                        ip = %ip,
                        base = base_oid_str,
                        attempt = transport_retries,
                        max_repetitions = max_reps,
                        getbulk = use_bulk,
                        error = %e,
                        "Re-issuing after no answer"
                    );
                    continue 'walk;
                }
                Err(e) => {
                    stop = WalkStop::Transport;
                    stop_detail = Some(e.to_string());
                    break;
                }
            }
        } else {
            match session.walk_getnext(&current_parts).await {
                Ok(v) => v,
                Err(e) if is_desync(&e) && desync_retries < MAX_DESYNC_RETRIES => {
                    desync_retries += 1;
                    debug!(
                        ip = %ip,
                        base = base_oid_str,
                        attempt = desync_retries,
                        error = %e,
                        "Re-issuing after reading a stale answer"
                    );
                    continue 'walk;
                }
                Err(e) if transport_retries < MAX_TRANSPORT_RETRIES => {
                    transport_retries += 1;
                    debug!(
                        ip = %ip,
                        base = base_oid_str,
                        attempt = transport_retries,
                        error = %e,
                        "Re-issuing after no answer"
                    );
                    continue 'walk;
                }
                Err(e) => {
                    stop = WalkStop::Transport;
                    stop_detail = Some(e.to_string());
                    break;
                }
            }
        };

        // Empty response mid-walk is abnormal (getbulk) or an exhausted column (getnext). Worth
        // one more ask before giving up on the column: every other wrong-shaped answer here — a
        // stale OID, a non-advancing OID, a request-id mismatch — is re-asked, and this one has
        // the same causes. An agent that means it answers the same way again and the column ends
        // as it did before.
        if varbinds.is_empty() {
            if transport_retries < MAX_TRANSPORT_RETRIES {
                transport_retries += 1;
                if use_bulk {
                    shrink_page(&mut max_reps, &mut use_bulk);
                }
                debug!(
                    ip = %ip,
                    base = base_oid_str,
                    attempt = transport_retries,
                    max_repetitions = max_reps,
                    getbulk = use_bulk,
                    "Re-issuing after an answer with no varbinds on it"
                );
                continue 'walk;
            }
            stop = WalkStop::EmptyResponse;
            break;
        }

        // Process the response, remembering the last in-subtree OID to continue from.
        let mut next_parts: Option<Vec<u64>> = None;
        let mut done = false;
        // Set when the agent answered with an OID belonging to some other question. Re-asking is
        // worth a try before giving up on the column.
        let mut retry_page = false;
        // Rows on this page the walk had not already collected. A page that contributes none is
        // the agent repeating itself, which is the one shape that cannot terminate on its own.
        let mut page_new = 0usize;
        for (resp_parts, value) in varbinds {
            if matches!(
                value,
                Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
            ) {
                stop = WalkStop::EndOfMibView;
                done = true;
                break;
            }
            if resp_parts.len() <= base_parts.len() || !resp_parts.starts_with(&base_parts) {
                // Out of the subtree. That is the natural end of a column *if* the agent moved
                // forward past it — a walk always advances. An OID that doesn't exceed where we
                // asked from is not a continuation of this walk at all (a stale response left
                // over from a cancelled request reads exactly like this), and calling it a
                // natural end would report a column that stopped early as authoritative, which
                // then re-enables the server-side prune #649 exists to suppress.
                if resp_parts <= high_water {
                    stop_detail = Some(format!("responded with {resp_parts:?}"));
                    stop = WalkStop::StaleResponse;
                    // Retryable for the same reason a request-id mismatch is: the answer belongs
                    // to an earlier question and has now been consumed, so re-asking gets a fresh
                    // one. Unlike that case there is no transport error — the response was valid
                    // and carried the wrong OID, which is what a forking `pass` handler under
                    // load produces.
                    retry_page = true;
                } else {
                    stop = WalkStop::EndOfSubtree;
                }
                done = true;
                break;
            }
            // In the subtree, so this is a row of the table being walked — whether or not it
            // ascends. Firmware that stores a table unsorted serves real rows in a real order
            // that simply is not numeric (GH #674); refusing them read part of the reporter's
            // switch and reported the rest as absent. Identity, not ordering, is what separates
            // that from an agent looping: a row already collected is a repeat and is dropped,
            // which also keeps a re-asked page from emitting its rows twice.
            if seen.insert(resp_parts.clone()) {
                if resp_parts > high_water {
                    high_water.clone_from(&resp_parts);
                }
                on_entry(&resp_parts[base_parts.len()..], &value);
                count += 1;
                page_new += 1;
            }
            // Continue from where the agent left off in its own order, which is what lets the
            // rest of an out-of-order table be reached at all.
            next_parts = Some(resp_parts);
            if count >= MAX_WALK_ENTRIES {
                stop = WalkStop::EntryCap;
                done = true;
                break;
            }
        }
        if !done {
            match next_parts {
                Some(parts) => {
                    // The walk must keep making progress — but progress is new rows, not a
                    // larger OID. A device that answers with a tail OID that doesn't
                    // lexicographically exceed the one we asked from (observed on Ubiquiti
                    // bridge-FDB) would otherwise have us re-request the same page until
                    // MAX_WALK_ENTRIES or the integration timeout; it still does, because its
                    // second identical page contributes nothing new. Testing the tail OID
                    // instead used to catch out-of-order firmware in the same net (#674),
                    // which had us discard rows that were there for the asking.
                    if page_new == 0 {
                        stop_detail = Some(format!("responded with {parts:?}"));
                        stop = WalkStop::NonAdvancingOid;
                        retry_page = true;
                        done = true;
                    } else {
                        current_parts = parts;
                    }
                }
                None => {
                    stop = WalkStop::EmptyResponse;
                    done = true;
                }
            }
        }

        // Both wrong-OID shapes converge here — the one detected inside the page and the one
        // detected on the tail — so a retry covers each.
        if retry_page && desync_retries < MAX_DESYNC_RETRIES {
            desync_retries += 1;
            debug!(
                ip = %ip,
                base = base_oid_str,
                attempt = desync_retries,
                ?stop,
                detail = stop_detail.as_deref().unwrap_or(""),
                "Re-asking after an answer that belonged to another request"
            );
            // Re-asking cannot duplicate rows, because `seen` drops any the callback already
            // took. It used to be able to: the wrong-OID varbind is rejected before the
            // callback, but the in-subtree ones ahead of it on the same page were not, and the
            // cursor had not moved — so a re-ask re-delivered them, pushing duplicate VLANs and
            // duplicate per-port memberships into the collectors that append rather than key.
            stop = WalkStop::EndOfSubtree;
            stop_detail = None;
            continue 'walk;
        }
        if done {
            break;
        }
    }

    // A truncated column is why interfaces and neighbours go missing, and the reason is otherwise
    // invisible — a timeout and a session reading stale answers produce identical data. Kept at
    // debug rather than info: truncation is common enough on a busy agent to be noise at info
    // (the SNMP simulator alone produces several per scan), and the operator-facing signal is
    // already the session warning. This is the follow-up detail for when that warning needs
    // explaining. A clean walk stays silent either way.
    if stop.is_truncation() {
        // `ip` is not decoration. This is the only line that says *why* a column came up
        // short, and without the address it cannot be tied to a device — an operator
        // grepping their daemon log for the switch named in the scan warning filtered out
        // every one of these, which is what made a Ubiquiti bridge-FDB failure take two
        // rounds of logs to narrow. Threading it as a parameter rather than relying on an
        // enclosing span means a new walk cannot be added without supplying it.
        debug!(
            ip = %ip,
            base = base_oid_str,
            ?stop,
            detail = stop_detail.as_deref().unwrap_or(""),
            entries = count,
            "SNMP walk truncated"
        );
    }

    Ok(stop)
}

/// Query system MIB information from a device.
///
/// `sysServices` and `ifNumber` are read here alongside the descriptive scalars because they are
/// what the device claims about itself: the bridge bit in the first says it switches, and the
/// second says how many interfaces to expect. Both are compared against what the walks actually
/// return, so a device that short-changes a collection can be reported rather than believed.
pub async fn query_system_info<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SystemInfo> {
    let mut info = SystemInfo::default();

    // Query each system OID
    let oids_to_query = [
        (oids::system::SYS_DESCR, "sysDescr"),
        (oids::system::SYS_OBJECT_ID, "sysObjectID"),
        (oids::system::SYS_NAME, "sysName"),
        (oids::system::SYS_LOCATION, "sysLocation"),
        (oids::system::SYS_CONTACT, "sysContact"),
        (oids::system::SYS_UPTIME, "sysUpTime"),
        (oids::system::SYS_SERVICES, "sysServices"),
        (oids::if_mib::IF_NUMBER, "ifNumber"),
    ];

    for (oid_str, name) in oids_to_query {
        match session.get_scalar(&oids::oid_parts(oid_str)).await {
            Ok(Some(value)) => {
                trace!("SNMP {} from {}: {:?}", name, ip, value);
                match name {
                    "sysDescr" => info.sys_descr = value_to_string(&value),
                    "sysObjectID" => info.sys_object_id = value_to_string(&value),
                    "sysName" => info.sys_name = value_to_string(&value),
                    "sysLocation" => info.sys_location = value_to_string(&value),
                    "sysContact" => info.sys_contact = value_to_string(&value),
                    "sysUpTime" => info.sys_uptime = value_to_u64(&value),
                    "sysServices" => info.sys_services = value_to_i32(&value),
                    "ifNumber" => info.if_number = value_to_i32(&value),
                    _ => {}
                }
            }
            Ok(None) => {
                debug!("SNMP GET {} returned nothing from {}", name, ip);
            }
            Err(e) => {
                debug!("SNMP GET {} failed from {}: {}", name, ip, e);
            }
        }
    }

    Ok(info)
}

/// Records from a multi-column SNMP walk, plus whether the walk actually saw everything.
///
/// Absent data is ambiguous on its own: "this device has no neighbour on that port" and "we failed
/// to read it" are both an empty record, and they call for opposite responses server-side — clear
/// the stored value, or keep it. Only the daemon can tell them apart, so the answer travels with
/// the data.
///
/// `Default` is deliberately `complete: false`. These queries run under `query_or_default`, so a
/// whole-query timeout yields the default — and an empty result from a query that never finished
/// must never be mistaken for a device authoritatively reporting nothing.
#[derive(Debug)]
pub struct SnmpCollection<T> {
    pub records: T,
    pub complete: bool,
    /// The agent does not implement this MIB at all — it answered `noSuchObject` rather than
    /// walking past the end of a table it has.
    ///
    /// A third state is needed because `complete: true` with no records is the daemon telling the
    /// server "this device authoritatively has nothing here", which the server acts on by
    /// clearing what it holds. That is right for a switch whose last LLDP neighbour went away and
    /// wrong for one that has no LLDP-MIB: on a Ubiquiti USW-Pro-Max, `1.0.8802.1.1.2.1.4.1`
    /// returns `No Such Object`, so an SNMP pass would erase the neighbours the UniFi controller
    /// integration had just written for the same switch — the two run in the same scan, in no
    /// fixed order, so the data would come and go between scans.
    ///
    /// Computed for the neighbour tables (LLDP, CDP), which are the columns another integration
    /// also writes and so the only ones where overwriting with an empty result destroys data.
    /// Left `false` elsewhere, where nothing consumes it.
    pub unsupported: bool,
    /// Why it came up short, for the operator-facing warning.
    ///
    /// Separate from `unsupported`, which gates a *data* decision (may this overwrite what the
    /// server holds?) rather than a reporting one. They agree where both are set; keeping them
    /// apart means a change to how something reads cannot silently change what is stored.
    pub reason: Option<ShortfallReason>,
    /// Rows the walk *read* and then had to throw away as unusable.
    ///
    /// Distinct from every other field here, which describe rows that were never read. A record
    /// the agent served but that is missing a mandatory identifier is a fault in the device's
    /// data, not in the read, and no rescan fixes it — so it needs saying separately or it
    /// reads to an operator as a transient (GH #668).
    ///
    /// Set by both neighbour tables: LLDP discards a record with no chassis ID, CDP one with no
    /// device id, for the same reason in both cases.
    pub discarded: usize,
    /// What the device led us to expect here, when it published anything.
    ///
    /// The rest of this struct describes the read from the inside — how far it got, why it
    /// stopped, what it threw away. This is the one field sourced from the device rather than
    /// from us, and it is what lets a scan say "the device told us to expect 23 and we read 1"
    /// instead of only "we did not finish".
    ///
    /// `None` wherever nothing was published, which is most groups: the claim is only as good as
    /// the scalar behind it, and inventing one would be worse than staying quiet.
    pub claim: Option<DeviceClaim>,
    /// What accounts for most of `discarded`, or `None` when nothing was discarded.
    ///
    /// The count alone cannot say whether a rescan will help, and the warning built from it
    /// asserted that it never would — true of a device serving malformed rows, false of a column
    /// that stopped early, and the two were indistinguishable to the operator (GH #668).
    pub discard_reason: Option<MalformedNeighbourReason>,
    /// The records' local-port keys are already `ifIndex` values, so the caller must not put
    /// them through `remap_lldp_local_ports`.
    ///
    /// Set only by the LLDP neighbour walk, and only when it read the neighbours from the
    /// LLDP-V2-MIB (GH #688). `lldpV2RemLocalIfIndex` is an ifIndex by definition, where the
    /// classic `lldpRemLocalPortNum` is a separate namespace that has to be translated through
    /// `lldpLocPortTable` — a table a V2-only agent does not serve. Remapping a V2 result would
    /// treat a real ifIndex as a port number: identity on most firmware, and wrong on exactly the
    /// vendors the remap exists for. `false` everywhere else, where nothing consumes it.
    pub local_port_is_if_index: bool,
}

impl<T: Default> SnmpCollection<T> {
    /// A collection the caller had no reason to attempt.
    ///
    /// Distinct from [`Default`], which means a query that ran and failed. Nothing was asked, so
    /// there is no shortfall to report and no reason to name — reporting one would put a warning
    /// on every device that simply had no neighbours to place.
    pub fn skipped() -> Self {
        Self {
            records: T::default(),
            complete: true,
            unsupported: false,
            reason: None,
            discarded: 0,
            discard_reason: None,
            claim: None,
            local_port_is_if_index: false,
        }
    }
}

impl<T> SnmpCollection<T> {
    /// The ordinary outcome of a multi-column walk: the records, plus whether every column
    /// finished and why the first one that didn't stopped.
    ///
    /// `unsupported` and the discard fields stay at their neutral values — a query that discards
    /// rows, or that can tell "no such MIB" from "implemented and empty", sets them itself.
    fn from_walk(records: T, shortfall: Shortfall) -> Self {
        Self {
            records,
            complete: shortfall.complete,
            unsupported: false,
            reason: shortfall.reason,
            discarded: 0,
            discard_reason: None,
            claim: None,
            local_port_is_if_index: false,
        }
    }
}

impl<T: Default> Default for SnmpCollection<T> {
    fn default() -> Self {
        Self {
            records: T::default(),
            complete: false,
            unsupported: false,
            discarded: 0,
            discard_reason: None,
            claim: None,
            local_port_is_if_index: false,
            // `query_or_default` produces this when a whole query timed out or errored, and it
            // genuinely cannot say more — the future was dropped before it could report.
            reason: Some(ShortfallReason::NoAnswer),
        }
    }
}

/// The outcome of walking the ifTable/ifXTable columns.
///
/// Two independent notions of "complete", because they answer different questions and only one of
/// them may gate a destructive operation. `ifIndex` is the table's index column: it alone decides
/// *which* interfaces exist. The other ten carry attributes of interfaces already known.
///
/// Collapsing both into one flag (as this used to) meant a timed-out `ifDescr` read blocked the
/// server-side prune — so stale interfaces lingered on any device with one flaky column — and
/// raised an operator warning about missing interfaces when none were missing.
#[derive(Default)]
pub struct IfTableWalk {
    pub entries: Vec<IfTableEntry>,
    /// Every interface the device listed is present. The set is authoritative, so the server may
    /// prune interfaces absent from it (#649). False whenever the `ifIndex` column itself was cut
    /// short, or a column answered for an interface the device never listed.
    pub set_complete: bool,
    /// Every attribute column also walked to its end. False means some descriptions, speeds or
    /// aliases may be blank — a cosmetic gap, never a reason to withhold pruning.
    pub attributes_complete: bool,
}

// `Default` is the hard-failure outcome (`query_or_default`): no entries, and neither flag set,
// so a walk that never ran can never be mistaken for an authoritative one.

/// Why a column walk stopped.
///
/// Only the first two are a genuine end; the rest are truncation, and telling them apart is the
/// whole diagnostic. "The device is slow" (`Timeout`) and "this session is reading answers to
/// questions it already gave up on" (`SessionDesync`) look identical in the data — both just
/// produce a short column — but they call for completely different responses.
#[derive(Debug, Clone, Copy)]
enum WalkStop {
    /// Responses moved past the requested subtree — the column is finished.
    EndOfSubtree,
    /// Agent signalled end-of-MIB / no-such-object.
    EndOfMibView,
    /// Hit `MAX_WALK_ENTRIES`.
    EntryCap,
    /// getbulk/getnext returned an error. The message distinguishes a timeout from a
    /// request-id or community mismatch.
    Transport,
    /// Agent answered with no varbinds at all mid-walk.
    EmptyResponse,
    /// Agent answered with an OID that did not advance — it would loop for ever.
    NonAdvancingOid,
    /// Left the subtree without advancing: not this walk's continuation at all.
    StaleResponse,
}

impl From<WalkStop> for Option<ShortfallReason> {
    /// Collapse the walk's own vocabulary into the four things an operator can act on
    /// differently. The distinctions dropped here (`EmptyResponse` vs `Transport`) are
    /// diagnostic detail, already in the truncation log with the host address.
    fn from(stop: WalkStop) -> Self {
        match stop {
            WalkStop::EndOfSubtree => None,
            WalkStop::EndOfMibView => Some(ShortfallReason::Unsupported),
            WalkStop::EntryCap => Some(ShortfallReason::EntryCap {
                limit: MAX_WALK_ENTRIES,
            }),
            WalkStop::NonAdvancingOid | WalkStop::StaleResponse => {
                Some(ShortfallReason::Desynchronised)
            }
            WalkStop::Transport | WalkStop::EmptyResponse => Some(ShortfallReason::NoAnswer),
        }
    }
}

impl WalkStop {
    fn is_truncation(self) -> bool {
        !matches!(self, Self::EndOfSubtree | Self::EndOfMibView)
    }

    /// The walk reached the column's end and read everything the agent has.
    fn is_complete(self) -> bool {
        !self.is_truncation()
    }

    /// The agent answered "I do not have this OID" rather than walking past the end of a table
    /// it implements.
    ///
    /// These are different answers and the difference matters: an implemented-but-empty table
    /// walks forward out of its own subtree ([`Self::EndOfSubtree`]), while an unimplemented MIB
    /// returns `noSuchObject` / `endOfMibView` at the first request ([`Self::EndOfMibView`]). Both
    /// yield zero rows, and only the first is a device authoritatively reporting "nothing here".
    fn is_unsupported(self) -> bool {
        matches!(self, Self::EndOfMibView)
    }
}

/// What a multi-column query managed across all its columns.
///
/// Replaces a bare `&mut bool`: the flag alone said *that* something fell short and the reason
/// stopped at the walk, so the operator-facing line had to guess — it claimed a query "usually
/// timed out" whether it had hit our entry cap, been answered out of step, or found a MIB the
/// device does not implement.
#[derive(Debug, Clone, Copy)]
pub struct Shortfall {
    pub complete: bool,
    pub reason: Option<ShortfallReason>,
}

impl Default for Shortfall {
    fn default() -> Self {
        Self {
            complete: true,
            reason: None,
        }
    }
}

impl Shortfall {
    /// Fold in one column's stop.
    ///
    /// First reason wins. Columns are walked in order and a session that has gone wrong tends to
    /// stay wrong, so the first failure is the one that explains the rest — a later `NoAnswer`
    /// on a session already desynchronised is a consequence, not a second finding.
    fn record(&mut self, stop: WalkStop) {
        if stop.is_complete() {
            return;
        }
        self.complete = false;
        if self.reason.is_none() {
            self.reason = Option::<ShortfallReason>::from(stop);
        }
    }
}

/// Walk one column, folding its outcome into `shortfall`.
///
/// Every multi-column query needs this and none of them had it: `walk_subtree` never returns
/// `Err`, so the `?` these call sites used was dead code and a truncated column was invisible.
async fn walk_column<T, F>(
    session: &mut T,
    ip: IpAddr,
    base_oid_str: &str,
    shortfall: &mut Shortfall,
    on_entry: F,
) -> WalkStop
where
    T: SnmpWalkTransport,
    F: FnMut(&[u64], &Value),
{
    let stop = walk_subtree(session, ip, base_oid_str, on_entry)
        .await
        .unwrap_or(WalkStop::Transport);
    shortfall.record(stop);
    stop
}

/// Walk the ifTable/ifXTable columns.
///
/// See [`IfTableWalk`] for what the two completeness flags mean and why they are separate.
pub async fn walk_if_table<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<IfTableWalk> {
    let mut entries: HashMap<i32, IfTableEntry> = HashMap::new();
    // Cleared to false the moment any column walk is cut short (error/timeout/limit).
    let mut shortfall = Shortfall::default();
    // Whether the index column specifically survived. `None` until it has been walked.
    let mut index_column_complete: Option<bool> = None;

    // Define the columns we want to walk
    let columns = [
        (oids::if_mib::columns::IF_INDEX, "ifIndex"),
        (oids::if_mib::columns::IF_DESCR, "ifDescr"),
        (oids::if_mib::columns::IF_TYPE, "ifType"),
        (oids::if_mib::columns::IF_MTU, "ifMtu"),
        (oids::if_mib::columns::IF_SPEED, "ifSpeed"),
        (oids::if_mib::columns::IF_PHYS_ADDRESS, "ifPhysAddress"),
        (oids::if_mib::columns::IF_ADMIN_STATUS, "ifAdminStatus"),
        (oids::if_mib::columns::IF_OPER_STATUS, "ifOperStatus"),
        (oids::if_mib::if_x_table::IF_NAME, "ifName"),
        (oids::if_mib::if_x_table::IF_HIGH_SPEED, "ifHighSpeed"),
        (oids::if_mib::if_x_table::IF_ALIAS, "ifAlias"),
    ];

    // ifIndex is walked first and is the table's index column, so once it has returned a
    // non-empty set every later column must land inside it. A row appearing only in a later
    // column is not an interface this device reported — it is a response that doesn't belong to
    // this walk — and minting an interface from it is how a foreign port ended up on a switch.
    // Only trusted when the ifIndex column itself completed; a device that doesn't serve it at
    // all still gets the old permissive behaviour.
    let mut known_if_indexes: Option<HashSet<i32>> = None;
    let mut foreign_rows = 0usize;

    // Walk each column. ifTable/ifXTable are indexed by a single sub-id (ifIndex).
    for (base_oid_str, column_name) in columns {
        let known = known_if_indexes.clone();
        let mut column_indexes: HashSet<i32> = HashSet::new();
        let mut column_foreign = 0usize;
        let walked = walk_subtree(session, ip, base_oid_str, |suffix, value| {
            let Some(&if_index_u64) = suffix.last() else {
                return;
            };
            let if_index = if_index_u64 as i32;
            column_indexes.insert(if_index);
            if let Some(known) = &known
                && !known.contains(&if_index)
            {
                column_foreign += 1;
                return;
            }
            let entry = entries.entry(if_index).or_insert_with(|| IfTableEntry {
                if_index,
                if_descr: None,
                if_type: None,
                if_mtu: None,
                if_speed: None,
                if_phys_address: None,
                if_admin_status: None,
                if_oper_status: None,
                if_name: None,
                if_alias: None,
            });
            match column_name {
                "ifIndex" => {} // already set above
                "ifDescr" => entry.if_descr = value_to_string(value),
                "ifType" => entry.if_type = value_to_i32(value),
                "ifMtu" => entry.if_mtu = value_to_i32(value),
                "ifSpeed" => {
                    // Only set if ifHighSpeed not already set
                    if entry.if_speed.is_none() {
                        entry.if_speed = value_to_u64(value);
                    }
                }
                "ifPhysAddress" => entry.if_phys_address = value_to_mac(value),
                "ifAdminStatus" => entry.if_admin_status = value_to_i32(value),
                "ifOperStatus" => entry.if_oper_status = value_to_i32(value),
                "ifName" => entry.if_name = value_to_string(value),
                "ifHighSpeed" => {
                    // ifHighSpeed is in Mbps, convert to bps for consistency
                    if let Some(mbps) = value_to_u64(value) {
                        entry.if_speed = Some(mbps * 1_000_000);
                    }
                }
                "ifAlias" => entry.if_alias = value_to_string(value),
                _ => {}
            }
        })
        .await
        .map(WalkStop::is_complete)
        .unwrap_or(false);

        // A column cut short (timeout/error/limit) means this is NOT an authoritative
        // full ifTable — the server must not prune stale interfaces against it (#649).
        if !walked {
            shortfall.complete = false;
        }

        if column_name == "ifIndex" {
            index_column_complete = Some(walked);
            // A column cut short still names the indexes it *did* return, and a row outside that
            // set is not an interface this device listed — truncated or not. Only a column that
            // returned nothing leaves no basis to judge, and that is the sole case that falls back
            // to accepting whatever the other columns mint. Requiring the column to have finished
            // let a foreign ifIndex through on exactly the scan where the guard was needed most.
            if !column_indexes.is_empty() {
                known_if_indexes = Some(column_indexes);
            }
        }

        if column_foreign > 0 {
            // Something answered for an interface this device never listed. Whatever the cause,
            // what we hold is not a faithful copy of its ifTable.
            foreign_rows += column_foreign;
            shortfall.complete = false;
            tracing::warn!(
                ip = %ip,
                column = column_name,
                rows = column_foreign,
                "SNMP ifTable column returned rows for unknown ifIndexes; discarding them and \
                 marking the walk partial"
            );
        }
    }

    let mut result: Vec<IfTableEntry> = entries.into_values().collect();
    result.sort_by_key(|e| e.if_index);

    // The ifTable keeps its own two-flag model (`set_complete` / `attributes_complete`) rather
    // than reporting a `ShortfallReason`: a truncated interface *set* and a truncated attribute
    // *column* mean different things to the server, and only the first may gate pruning. The
    // accumulator is used here purely for the attribute-column flag.
    let complete = shortfall.complete;

    // A foreign row means something answered for an interface this device never listed, so the
    // set itself is suspect — not just its attributes.
    let set_complete = match index_column_complete {
        // The index column decides membership, so its own completeness is the set's.
        Some(true) => foreign_rows == 0,
        Some(false) => false,
        // A device that serves no index column at all gives us no independent read on membership;
        // fall back to requiring every column, which is what this did before the split.
        None => complete,
    };

    // `complete` distinguishes an authoritative full ifTable from a partial walk cut short by
    // timeout/error. The server prunes stale interfaces only on a complete walk (GH #649), so
    // surface it at debug level for self-hosted daemon-log triage (enable SCANOPY_LOG_LEVEL=debug).
    tracing::debug!(
        ip = %ip,
        if_count = result.len(),
        set_complete = set_complete,
        attributes_complete = complete,
        foreign_rows = foreign_rows,
        "SNMP ifTable walk finished"
    );
    // Diagnostic for issue #614 (high-ifIndex interfaces missing): log the full set of
    // collected ifIndex values, not just the count, so we can tell whether a high-ifIndex
    // switch (e.g. ifIndex 49153-49168) is dropped at walk time or later during ingestion.
    debug!(
        ip = %ip,
        if_indexes = ?result.iter().map(|e| e.if_index).collect::<Vec<_>>(),
        "SNMP ifTable walk ifIndex set"
    );

    Ok(IfTableWalk {
        entries: result,
        set_complete,
        attributes_complete: complete,
    })
}

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
fn split_lldp_rem_index(suffix: &[u64]) -> Option<(i32, i32)> {
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
fn split_lldp_v2_rem_index(suffix: &[u64]) -> Option<(i32, i32)> {
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
fn split_lldp_v2_man_addr_index(suffix: &[u64]) -> Option<(i32, i32, Vec<u8>)> {
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

/// Query ipAddrTable for IP address to ifIndex + subnet mask mappings.
/// Walks ipAdEntIfIndex and ipAdEntNetMask columns where the OID suffix
/// encodes the IP address as A.B.C.D.
pub async fn query_ip_addr_table<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<HashMap<IpAddr, IpAddrEntry>>> {
    let mut if_index_map: HashMap<IpAddr, i32> = HashMap::new();
    let mut net_mask_map: HashMap<IpAddr, IpAddr> = HashMap::new();
    let mut shortfall = Shortfall::default();

    // Walk ipAdEntIfIndex — OID suffix encodes the IP address as A.B.C.D.
    walk_column(
        session,
        ip,
        oids::ip_mib::ip_addr_entry::IP_AD_ENT_IF_INDEX,
        &mut shortfall,
        |suffix, value| {
            if suffix.len() == 4
                && let Some(if_index) = value_to_i32(value)
            {
                let addr = IpAddr::from([
                    suffix[0] as u8,
                    suffix[1] as u8,
                    suffix[2] as u8,
                    suffix[3] as u8,
                ]);
                if_index_map.insert(addr, if_index);
            }
        },
    )
    .await;

    // Walk ipAdEntNetMask
    walk_column(
        session,
        ip,
        oids::ip_mib::ip_addr_entry::IP_AD_ENT_NET_MASK,
        &mut shortfall,
        |suffix, value| {
            if suffix.len() == 4
                && let Some(mask) = value_to_ip(value)
            {
                let addr = IpAddr::from([
                    suffix[0] as u8,
                    suffix[1] as u8,
                    suffix[2] as u8,
                    suffix[3] as u8,
                ]);
                net_mask_map.insert(addr, mask);
            }
        },
    )
    .await;

    // Combine ifIndex and netMask results
    let result: HashMap<IpAddr, IpAddrEntry> = if_index_map
        .into_iter()
        .map(|(addr, if_index)| {
            let net_mask = net_mask_map.get(&addr).copied();
            (addr, IpAddrEntry { if_index, net_mask })
        })
        .collect();

    debug!(
        "ipAddrTable walk from {} returned {} entries",
        ip,
        result.len()
    );

    Ok(SnmpCollection::from_walk(result, shortfall))
}

/// Query CDP cache table for neighbor information (Cisco devices)
pub async fn query_cdp_neighbors<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Vec<CdpNeighbor>>> {
    let mut neighbors: HashMap<(i32, i32), CdpNeighbor> = HashMap::new();
    let mut shortfall = Shortfall::default();

    let columns = [
        (oids::cdp::entry::CDP_CACHE_DEVICE_ID, "deviceId"),
        (oids::cdp::entry::CDP_CACHE_DEVICE_PORT, "devicePort"),
        (oids::cdp::entry::CDP_CACHE_PLATFORM, "platform"),
        (oids::cdp::entry::CDP_CACHE_ADDRESS, "address"),
    ];

    // See `query_lldp_neighbors`: the same distinction, for the same reason. CDP-MIB is a Cisco
    // enterprise MIB, so the overwhelming majority of devices answer `noSuchObject` for it.
    let mut all_columns_unsupported = true;

    for (base_oid_str, column_name) in columns {
        // CDP index: cdpCacheIfIndex.cdpCacheDeviceIndex
        let stop = walk_column(
            session,
            ip,
            base_oid_str,
            &mut shortfall,
            |suffix, value| {
                if suffix.len() < 2 {
                    return;
                }
                let if_index = suffix[0] as i32;
                let device_index = suffix[1] as i32;
                let neighbor = neighbors
                    .entry((if_index, device_index))
                    .or_insert_with(|| CdpNeighbor {
                        local_port_index: if_index,
                        remote_device_id: None,
                        remote_port_id: None,
                        remote_platform: None,
                        remote_address: None,
                    });
                match column_name {
                    "deviceId" => neighbor.remote_device_id = value_to_string(value),
                    "devicePort" => neighbor.remote_port_id = value_to_string(value),
                    "platform" => neighbor.remote_platform = value_to_string(value),
                    "address" => {
                        // CDP address is encoded as 4 bytes for IPv4
                        if let Value::OctetString(bytes) = value
                            && bytes.len() == 4
                        {
                            neighbor.remote_address =
                                Some(IpAddr::from([bytes[0], bytes[1], bytes[2], bytes[3]]));
                        }
                    }
                    _ => {}
                }
            },
        )
        .await;
        if !stop.is_unsupported() {
            all_columns_unsupported = false;
        }
    }

    // cdpCacheDeviceId is what L2 resolution matches on, so a record without one is the CDP
    // analogue of a chassis-less LLDP neighbour: unusable, and destructive if it overwrites.
    let before = neighbors.len();
    let result: Vec<CdpNeighbor> = neighbors
        .into_values()
        .filter(|n| n.remote_device_id.is_some())
        .collect();
    // CDP has no separate index column to compare against, so the walk's own outcome is the only
    // evidence of why the id is absent: a column that stopped early can recover on a rescan, and a
    // walk that ran to the end and still produced idless rows never will.
    let discard_reason = (result.len() != before).then(|| {
        if shortfall.reason.is_some() {
            MalformedNeighbourReason::WalkCutShort
        } else {
            MalformedNeighbourReason::GhostRows
        }
    });
    if result.len() != before {
        shortfall.complete = false;
        warn!(
            ip = %ip,
            dropped = before - result.len(),
            reason = ?discard_reason,
            "CDP neighbours missing a device id; discarding them and marking the walk partial"
        );
    }
    let unsupported = all_columns_unsupported && result.is_empty();
    debug!(
        ip = %ip,
        neighbors = result.len(),
        complete = shortfall.complete,
        unsupported,
        "CDP query finished"
    );

    Ok(SnmpCollection {
        discarded: before - result.len(),
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

/// Query ARP table (ipNetToMediaTable) for IP-to-MAC mappings.
/// Returns entries with ifIndex, MAC, and IP for each ARP cache entry.
pub async fn query_arp_table<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Vec<ArpEntry>>> {
    // We need to walk 4 columns: ifIndex, physAddress, netAddress, type
    // OID suffix format: ifIndex.A.B.C.D
    struct ArpEntryBuilder {
        if_index: Option<i32>,
        mac_address: Option<mac_address::MacAddress>,
        ip_address: Option<IpAddr>,
        entry_type: Option<i32>,
    }

    let mut entries: HashMap<String, ArpEntryBuilder> = HashMap::new();
    let mut shortfall = Shortfall::default();

    let columns = [
        (oids::arp::entry::IP_NET_TO_MEDIA_IF_INDEX, "ifIndex"),
        (
            oids::arp::entry::IP_NET_TO_MEDIA_PHYS_ADDRESS,
            "physAddress",
        ),
        (oids::arp::entry::IP_NET_TO_MEDIA_NET_ADDRESS, "netAddress"),
        (oids::arp::entry::IP_NET_TO_MEDIA_TYPE, "type"),
    ];

    for (base_oid_str, column_name) in columns {
        // OID suffix: ifIndex.A.B.C.D
        walk_column(
            session,
            ip,
            base_oid_str,
            &mut shortfall,
            |suffix, value| {
                if suffix.len() < 5 {
                    return;
                }
                let key = suffix
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(".");
                let entry = entries.entry(key).or_insert_with(|| ArpEntryBuilder {
                    if_index: None,
                    mac_address: None,
                    ip_address: None,
                    entry_type: None,
                });
                match column_name {
                    "ifIndex" => entry.if_index = value_to_i32(value),
                    "physAddress" => entry.mac_address = value_to_mac(value),
                    "netAddress" => entry.ip_address = value_to_ip(value),
                    "type" => entry.entry_type = value_to_i32(value),
                    _ => {}
                }
            },
        )
        .await;
    }

    let rows_read = entries.len();

    // Filter out invalid entries (type==2) and entries missing required fields
    let result: Vec<ArpEntry> = entries
        .into_values()
        .filter_map(|e| {
            let entry_type = e.entry_type.unwrap_or(0);
            // Skip invalid entries (type 2)
            if entry_type == 2 {
                return None;
            }
            Some(ArpEntry {
                if_index: e.if_index?,
                mac_address: e.mac_address?,
                ip_address: e.ip_address?,
            })
        })
        .collect();

    // `rows_read` alongside the result is what makes an empty ARP table diagnosable. The entry is
    // a join across four columns and needs all of them, so a column that comes back empty drops
    // every row the others read — reported as "no ARP entries" from a device that answered
    // hundreds of them (GH #674). The two numbers together say which happened.
    debug!(
        ip = %ip,
        entries = result.len(),
        rows_read,
        complete = shortfall.complete,
        "ARP table walk finished"
    );

    Ok(SnmpCollection::from_walk(result, shortfall))
}

/// Query ENTITY-MIB entPhysicalTable for hardware inventory.
/// Returns the best-match physical entity (chassis > stack > module).
pub async fn query_entity_physical<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Option<DeviceInventory>>> {
    struct PhysicalEntry {
        description: Option<String>,
        class: Option<i32>,
        name: Option<String>,
        serial_number: Option<String>,
        manufacturer: Option<String>,
        model: Option<String>,
    }

    let mut entries: HashMap<i32, PhysicalEntry> = HashMap::new();
    let mut shortfall = Shortfall::default();

    let columns = [
        (oids::entity::entry::ENT_PHYSICAL_DESCR, "descr"),
        (oids::entity::entry::ENT_PHYSICAL_CLASS, "class"),
        (oids::entity::entry::ENT_PHYSICAL_NAME, "name"),
        (oids::entity::entry::ENT_PHYSICAL_SERIAL_NUM, "serialNum"),
        (oids::entity::entry::ENT_PHYSICAL_MFG_NAME, "mfgName"),
        (oids::entity::entry::ENT_PHYSICAL_MODEL_NAME, "modelName"),
    ];

    for (base_oid_str, column_name) in columns {
        // OID suffix is entPhysicalIndex (single integer).
        walk_column(
            session,
            ip,
            base_oid_str,
            &mut shortfall,
            |suffix, value| {
                let Some(&index_u64) = suffix.last() else {
                    return;
                };
                let entry = entries
                    .entry(index_u64 as i32)
                    .or_insert_with(|| PhysicalEntry {
                        description: None,
                        class: None,
                        name: None,
                        serial_number: None,
                        manufacturer: None,
                        model: None,
                    });
                match column_name {
                    "descr" => entry.description = value_to_string(value),
                    "class" => entry.class = value_to_i32(value),
                    "name" => entry.name = value_to_string(value),
                    "serialNum" => {
                        entry.serial_number = value_to_string(value).filter(|s| !s.is_empty())
                    }
                    "mfgName" => {
                        entry.manufacturer = value_to_string(value).filter(|s| !s.is_empty())
                    }
                    "modelName" => entry.model = value_to_string(value).filter(|s| !s.is_empty()),
                    _ => {}
                }
            },
        )
        .await;
    }

    // Select best match: prefer chassis (3), fallback to stack (11), then module (9)
    let best = entries
        .values()
        .find(|e| e.class == Some(3))
        .or_else(|| entries.values().find(|e| e.class == Some(11)))
        .or_else(|| entries.values().find(|e| e.class == Some(9)));

    let result = best.map(|e| DeviceInventory {
        description: e.description.clone().or_else(|| e.name.clone()),
        manufacturer: e.manufacturer.clone(),
        model: e.model.clone(),
        serial_number: e.serial_number.clone(),
    });

    debug!(
        "ENTITY-MIB query from {} returned {} physical entries, best match: {}",
        ip,
        entries.len(),
        result.is_some()
    );

    Ok(SnmpCollection::from_walk(result, shortfall))
}

/// Walk dot1dBasePortIfIndex to build bridge_port → ifIndex mapping.
///
/// This is the highest-leverage truncation in the file: both FDB and VLAN-membership collection
/// key everything off it, so a cut-short walk here silently empties both for the whole switch.
///
/// Walked **once per host** by the caller and handed to both consumers. It used to be walked
/// independently inside each of them, which on a device that answers this OID with silence
/// rather than `noSuchObject` — the Ubiquiti USW-Pro-Max does exactly this — paid the full
/// walk timeout twice per scan for a table that was never going to arrive.
pub async fn query_bridge_port_mapping<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<HashMap<i32, i32>>> {
    let mut port_to_if_index: HashMap<i32, i32> = HashMap::new();
    let mut shortfall = Shortfall::default();

    // Asked before the walk so the device's own figure survives a walk that returns nothing —
    // which is the case worth reporting. A switch declaring 48 bridge ports and then answering
    // this table with silence has contradicted itself, and that reads very differently to an
    // operator than the bare "does not implement SNMP bridge-port numbering" it produces today.
    let claim = session
        .get_scalar(&oids::oid_parts(oids::bridge::DOT1D_BASE_NUM_PORTS))
        .await
        .ok()
        .flatten()
        .and_then(|value| value_to_i32(&value))
        .filter(|ports| *ports > 0)
        .map(|ports| DeviceClaim::Count {
            source: ClaimSource::Dot1dBaseNumPorts,
            expected: ports as usize,
        });

    // OID suffix is the bridge port number; value is the ifIndex.
    walk_column(
        session,
        ip,
        oids::bridge::DOT1D_BASE_PORT_IF_INDEX,
        &mut shortfall,
        |suffix, value| {
            if let Some(&port_u64) = suffix.last()
                && let Some(if_index) = value_to_i32(value)
            {
                port_to_if_index.insert(port_u64 as i32, if_index);
            }
        },
    )
    .await;

    Ok(SnmpCollection {
        records: port_to_if_index,
        complete: shortfall.complete,
        unsupported: false,
        reason: shortfall.reason,
        discarded: 0,
        discard_reason: None,
        claim,
        local_port_is_if_index: false,
    })
}

/// In-progress FDB row assembled column-by-column across an SNMP walk, keyed by
/// its MAC. Shared by the legacy (dot1dTpFdbTable) and VLAN-aware (dot1qTpFdbTable)
/// collectors so their results can be merged by MAC.
#[derive(Default)]
struct FdbBuilder {
    mac_address: Option<mac_address::MacAddress>,
    port: Option<i32>,
    status: Option<i32>,
}

/// Query bridge FDB for MAC-to-port mappings, resolving bridge ports to ifIndex
/// values via dot1dBasePortIfIndex. Collects both the legacy `dot1dTpFdbTable`
/// (RFC 4188) and the VLAN-aware `dot1qTpFdbTable` (Q-BRIDGE, RFC 4363) — many
/// VLAN-aware switches (Aruba/HP ProCurve, etc.) populate only the latter and
/// leave the legacy table empty, so relying on dot1d alone silently produced no
/// L2 adjacency for them (GH #649).
pub async fn query_bridge_fdb<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
    // Step 1 (`query_bridge_port_mapping`) is done by the caller and shared with
    // `query_port_vlan_membership`. Both FDB tables reference this same dot1dBasePort space.
    bridge_ports: &SnmpCollection<HashMap<i32, i32>>,
) -> Result<SnmpCollection<Vec<BridgeFdbEntry>>> {
    // Seeded from the shared bridge-port walk: when *that* failed, everything keyed by it
    // inherits the reason rather than inventing one of its own.
    let mut shortfall = Shortfall {
        complete: bridge_ports.complete,
        reason: bridge_ports.reason,
    };
    let port_to_if_index = &bridge_ports.records;

    // Step 2: Walk legacy dot1dTpFdbTable columns.
    let mut fdb_entries: HashMap<String, FdbBuilder> = HashMap::new();

    // Every column answering "no such object" is a device with no bridge MIB *here* — which on a
    // switch that partitions its forwarding database by VLAN means "not in this context", not
    // "this switch forwards nothing". `unsupported` was hard-coded false, so a Catalyst read
    // without its VLAN context reported a complete, empty, authoritative read of a table it had
    // never been asked for, and the operator was told nothing at all (GH #686).
    let mut all_columns_unsupported = true;

    let columns = [
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_ADDRESS, "address"),
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_PORT, "port"),
        (oids::bridge::fdb_entry::DOT1D_TP_FDB_STATUS, "status"),
    ];

    for (base_oid_str, column_name) in columns {
        // OID suffix is a 6-octet MAC encoded as 6 sub-ids.
        let stop = walk_column(
            session,
            ip,
            base_oid_str,
            &mut shortfall,
            |suffix, value| {
                if suffix.len() != 6 {
                    return;
                }
                let key = suffix
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(".");
                let entry = fdb_entries.entry(key).or_default();
                match column_name {
                    "address" => entry.mac_address = value_to_mac(value),
                    "port" => entry.port = value_to_i32(value),
                    "status" => entry.status = value_to_i32(value),
                    _ => {}
                }
            },
        )
        .await;
        if !stop.is_unsupported() {
            all_columns_unsupported = false;
        }
    }

    // Step 3: Merge in VLAN-aware Q-BRIDGE dot1qTpFdbTable entries. Legacy rows
    // win; Q-BRIDGE fills in MACs the legacy table didn't report (or all of them,
    // on switches that populate only the Q-BRIDGE table).
    let qbridge = walk_qbridge_fdb(session, ip).await.unwrap_or_default();
    if !qbridge.complete {
        shortfall.complete = false;
    }
    if !qbridge.unsupported {
        all_columns_unsupported = false;
    }
    let qbridge = qbridge.records;
    for (key, builder) in qbridge {
        fdb_entries.entry(key).or_insert(builder);
    }

    // Filter: keep learned(3) and mgmt(5), resolve bridge port to ifIndex. Not self(4) — those
    // are the bridge's own port addresses, which name no neighbour. (The old comment here read
    // "self (5)"; 5 is mgmt, per the encoding documented on DOT1Q_TP_FDB_STATUS.)
    let result: Vec<BridgeFdbEntry> = fdb_entries
        .into_values()
        .filter_map(|e| {
            let status = e.status.unwrap_or(0);
            if status != 3 && status != 5 {
                return None;
            }
            let bridge_port = e.port?;
            Some(BridgeFdbEntry {
                mac_address: e.mac_address?,
                bridge_port,
                if_index: port_to_if_index.get(&bridge_port).copied(),
                status,
            })
        })
        .collect();

    tracing::debug!(
        ip = %ip,
        entries = result.len(),
        port_mappings = port_to_if_index.len(),
        complete = shortfall.complete,
        "Bridge FDB walk finished"
    );

    // Only when neither table was there to read. A device serving one row is reporting one row;
    // a device serving neither table is not reporting at all.
    let unsupported = all_columns_unsupported && result.is_empty();

    Ok(SnmpCollection {
        records: result,
        complete: shortfall.complete,
        unsupported,
        reason: shortfall.reason,
        discarded: 0,
        discard_reason: None,
        claim: None,
        local_port_is_if_index: false,
    })
}

/// Walk the VLAN-aware Q-BRIDGE FDB (`dot1qTpFdbTable`, RFC 4363) for MAC→port
/// mappings, keyed by MAC so results merge with the legacy `dot1dTpFdbTable`.
///
/// Unlike the legacy table, the MAC lives in the table INDEX
/// (`dot1qFdbId` + 6 MAC octets), not a column, so it's derived from the OID
/// suffix. Ports are `dot1dBasePort` numbers, resolved by the caller against the
/// same `dot1dBasePortIfIndex` map. VLAN-aware switches (Aruba/HP ProCurve, etc.)
/// often populate only this table (GH #649).
async fn walk_qbridge_fdb<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<HashMap<String, FdbBuilder>>> {
    let mut entries: HashMap<String, FdbBuilder> = HashMap::new();
    let mut shortfall = Shortfall::default();
    // Reported so the caller can tell a switch with no Q-BRIDGE MIB from one whose Q-BRIDGE
    // table is genuinely empty. Hard-coding this false made the caller's own check dead.
    let mut all_columns_unsupported = true;

    let columns = [
        (oids::bridge::q_fdb_entry::DOT1Q_TP_FDB_PORT, "port"),
        (oids::bridge::q_fdb_entry::DOT1Q_TP_FDB_STATUS, "status"),
    ];

    for (base_oid_str, column_name) in columns {
        // Q-BRIDGE index = dot1qFdbId (1 sub-id) + MAC (6 octets).
        let stop = walk_column(
            session,
            ip,
            base_oid_str,
            &mut shortfall,
            |suffix, value| {
                let Some(mac) = qbridge_fdb_index_to_mac(suffix) else {
                    return;
                };
                if suffix.len() < 7 {
                    return;
                }
                // Key by MAC alone (drop fdb_id) so the same MAC learned across VLANs
                // collapses to one entry and merges with the legacy table's MAC key.
                let key = suffix[1..7]
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(".");
                let entry = entries.entry(key).or_default();
                entry.mac_address = Some(mac);
                match column_name {
                    "port" => entry.port = value_to_i32(value),
                    "status" => entry.status = value_to_i32(value),
                    _ => {}
                }
            },
        )
        .await;
        if !stop.is_unsupported() {
            all_columns_unsupported = false;
        }
    }

    let entries_empty = entries.is_empty();

    Ok(SnmpCollection {
        records: entries,
        complete: shortfall.complete,
        unsupported: all_columns_unsupported && entries_empty,
        reason: shortfall.reason,
        discarded: 0,
        discard_reason: None,
        claim: None,
        local_port_is_if_index: false,
    })
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

/// Query VLAN table for VLAN IDs and names.
/// Tries Q-BRIDGE dot1qVlanStaticName first, falls back to Cisco VTP vtpVlanName.
pub async fn query_vlan_table<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Vec<VlanInfo>>> {
    let mut vlans: Vec<VlanInfo> = Vec::new();
    let mut shortfall = Shortfall::default();

    // Try Q-BRIDGE dot1qVlanStaticName first. OID suffix is the VLAN ID.
    walk_column(
        session,
        ip,
        oids::vlan::q_bridge::DOT1Q_VLAN_STATIC_NAME,
        &mut shortfall,
        |suffix, value| {
            if let Some(&vlan_u64) = suffix.last()
                && let Some(name) = value_to_string(value)
            {
                vlans.push(VlanInfo {
                    vlan_id: vlan_u64 as u16,
                    name,
                });
            }
        },
    )
    .await;

    // Fall back to Cisco VTP if Q-BRIDGE returned nothing. VTP index is
    // mgmtDomainIndex.vlanId — use the last sub-id as the VLAN ID.
    if vlans.is_empty() {
        // A device with no Q-BRIDGE VLAN names has not fallen short if VTP answers instead, so
        // the fallback starts the reckoning again rather than inheriting the first walk's stop.
        shortfall = Shortfall::default();
        walk_column(
            session,
            ip,
            oids::vlan::cisco_vtp::VTP_VLAN_NAME,
            &mut shortfall,
            |suffix, value| {
                if let Some(&vlan_u64) = suffix.last()
                    && let Some(name) = value_to_string(value)
                {
                    vlans.push(VlanInfo {
                        vlan_id: vlan_u64 as u16,
                        name,
                    });
                }
            },
        )
        .await;
    }

    debug!(
        "VLAN table query from {} returned {} entries (Q-BRIDGE or VTP)",
        ip,
        vlans.len()
    );

    Ok(SnmpCollection::from_walk(vlans, shortfall))
}

/// Query per-port VLAN membership from Q-BRIDGE-MIB.
/// Uses dot1qPvid for native VLANs and dot1qVlanCurrentEgressPorts/UntaggedPorts
/// for tagged VLAN membership. Resolves bridge ports to ifIndex.
pub async fn query_port_vlan_membership<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
    // Step 1 (`query_bridge_port_mapping`) is done by the caller and shared with
    // `query_bridge_fdb`; every result below is keyed by bridge port.
    bridge_ports: &SnmpCollection<HashMap<i32, i32>>,
) -> Result<SnmpCollection<Vec<PortVlanMembership>>> {
    // Seeded from the shared bridge-port walk: when *that* failed, everything keyed by it
    // inherits the reason rather than inventing one of its own.
    let mut shortfall = Shortfall {
        complete: bridge_ports.complete,
        reason: bridge_ports.reason,
    };
    let port_to_if_index = &bridge_ports.records;

    if port_to_if_index.is_empty() {
        debug!(
            "No bridge port mappings from {} — skipping VLAN membership",
            ip
        );
        return Ok(SnmpCollection {
            records: Vec::new(),
            complete: shortfall.complete,
            unsupported: false,
            reason: shortfall.reason,
            discarded: 0,
            discard_reason: None,
            claim: None,
            local_port_is_if_index: false,
        });
    }

    // Step 2: Walk dot1qPvid for native VLAN per bridge port. OID suffix is the
    // bridge port number; value is the native VLAN ID.
    let mut native_vlans: HashMap<i32, u16> = HashMap::new();
    walk_column(
        session,
        ip,
        oids::vlan::q_bridge::DOT1Q_PVID,
        &mut shortfall,
        |suffix, value| {
            if let Some(&port_u64) = suffix.last()
                && let Some(vlan_id) = value_to_u16(value)
            {
                native_vlans.insert(port_u64 as i32, vlan_id);
            }
        },
    )
    .await;

    // Step 3: Walk dot1qVlanCurrentEgressPorts — PortList bitmap per VLAN, indexed
    // by timeFilter.vlanId (last sub-id is the VLAN ID).
    let mut egress_by_port: HashMap<i32, Vec<u16>> = HashMap::new();
    walk_column(
        session,
        ip,
        oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_EGRESS_PORTS,
        &mut shortfall,
        |suffix, value| {
            if let Some(&vlan_u64) = suffix.last()
                && let Value::OctetString(bytes) = value
            {
                let vlan_id = vlan_u64 as u16;
                for bp in parse_portlist_bitmap(bytes) {
                    egress_by_port.entry(bp).or_default().push(vlan_id);
                }
            }
        },
    )
    .await;

    // Step 4: Walk dot1qVlanCurrentUntaggedPorts — same bitmap format.
    let mut untagged_by_port: HashMap<i32, Vec<u16>> = HashMap::new();
    walk_column(
        session,
        ip,
        oids::vlan::q_bridge::DOT1Q_VLAN_CURRENT_UNTAGGED_PORTS,
        &mut shortfall,
        |suffix, value| {
            if let Some(&vlan_u64) = suffix.last()
                && let Value::OctetString(bytes) = value
            {
                let vlan_id = vlan_u64 as u16;
                for bp in parse_portlist_bitmap(bytes) {
                    untagged_by_port.entry(bp).or_default().push(vlan_id);
                }
            }
        },
    )
    .await;

    // Step 5: Assemble per-port membership, resolving bridge port → ifIndex
    let mut result: Vec<PortVlanMembership> = Vec::new();

    for (&bridge_port, &if_index) in port_to_if_index {
        let native_vlan = native_vlans.get(&bridge_port).copied();
        let egress_vlans = egress_by_port.get(&bridge_port);
        let untagged_vlans = untagged_by_port.get(&bridge_port);

        // Tagged VLANs = egress VLANs minus untagged VLANs for this port
        let tagged_vlans: Vec<u16> = match egress_vlans {
            Some(egress) => {
                let untagged_set: std::collections::HashSet<u16> = untagged_vlans
                    .map(|v| v.iter().copied().collect())
                    .unwrap_or_default();
                egress
                    .iter()
                    .copied()
                    .filter(|v| !untagged_set.contains(v))
                    .collect()
            }
            None => Vec::new(),
        };

        // Only include ports that have some VLAN data
        if native_vlan.is_some() || !tagged_vlans.is_empty() {
            result.push(PortVlanMembership {
                if_index,
                native_vlan,
                tagged_vlans,
            });
        }
    }

    debug!(
        "VLAN membership query from {} returned {} port memberships ({} bridge port mappings)",
        ip,
        result.len(),
        port_to_if_index.len()
    );

    Ok(SnmpCollection {
        records: result,
        complete: shortfall.complete,
        unsupported: false,
        reason: shortfall.reason,
        discarded: 0,
        discard_reason: None,
        claim: None,
        local_port_is_if_index: false,
    })
}

#[cfg(test)]
mod walk_tests {
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
                Err(Answer::TooBig) => Ok(WalkPage::TooBig),
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
        assert_eq!(seen, 1, "the one page it did answer is still collected");
        assert!(
            session.requests < 10,
            "the walk must give up on a silent device rather than spin on it (made {} requests)",
            session.requests
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
}

/// `walk_if_table` assembles one interface per ifIndex across eleven separate column walks, and
/// until now had no test at all — the multi-column assembly, the row-minting and the `complete`
/// aggregation were all uncovered, which is how a foreign interface ended up on a switch and was
/// still reported as an authoritative full ifTable.
#[cfg(test)]
mod if_table_tests {
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
            async fn walk_getbulk<'a>(
                &'a mut self,
                from: &[u64],
                max: u32,
            ) -> Result<WalkPage<'a>> {
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
            async fn walk_getbulk<'a>(
                &'a mut self,
                from: &[u64],
                max: u32,
            ) -> Result<WalkPage<'a>> {
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
            async fn walk_getbulk<'a>(
                &'a mut self,
                from: &[u64],
                max: u32,
            ) -> Result<WalkPage<'a>> {
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
            async fn walk_getbulk<'a>(
                &'a mut self,
                from: &[u64],
                max: u32,
            ) -> Result<WalkPage<'a>> {
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
            async fn walk_getbulk<'a>(
                &'a mut self,
                _from: &[u64],
                _max: u32,
            ) -> Result<WalkPage<'a>> {
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
            async fn walk_getbulk<'a>(
                &'a mut self,
                _from: &[u64],
                _max: u32,
            ) -> Result<WalkPage<'a>> {
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
            async fn walk_getbulk<'a>(
                &'a mut self,
                _from: &[u64],
                _max: u32,
            ) -> Result<WalkPage<'a>> {
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
    #[tokio::test]
    async fn a_persistently_desynced_session_still_reports_truncated() {
        struct AlwaysDesyncs;

        #[async_trait::async_trait]
        impl SnmpWalkTransport for AlwaysDesyncs {
            async fn walk_getbulk<'a>(
                &'a mut self,
                _from: &[u64],
                _max: u32,
            ) -> Result<WalkPage<'a>> {
                Err(anyhow::Error::new(snmp2::Error::RequestIdMismatch)
                    .context("SNMP session desynchronized"))
            }

            async fn walk_getnext<'a>(&'a mut self, _from: &[u64]) -> Result<Varbinds<'a>> {
                unreachable!("getbulk answers first")
            }
        }

        let stop = walk_subtree(&mut AlwaysDesyncs, ip(), IF_DESCR, |_, _| {})
            .await
            .unwrap();
        assert!(stop.is_truncation(), "got {stop:?}");
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
}

/// GH #674: an agent whose table rows are not in ascending OID order.
///
/// Firmware that stores a table unsorted and iterates it positionally answers GETNEXT with
/// whatever row comes next *in its own order*. That is what makes `snmpwalk` stop with "OID not
/// increasing" while `snmpbulkwalk -Cc` reads the same table in full: the rows are real and
/// retrievable, and only a client that insists every step ascend refuses them.
#[cfg(test)]
mod out_of_order_tests {
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
            async fn walk_getbulk<'a>(
                &'a mut self,
                _from: &[u64],
                max: u32,
            ) -> Result<WalkPage<'a>> {
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
}

#[cfg(test)]
mod lldp_v2_tests {
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
}
