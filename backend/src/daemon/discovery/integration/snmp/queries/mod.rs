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

mod arp;
mod bridge;
mod cdp;
mod entity;
mod lldp;
mod system;
mod walk;

pub use arp::query_arp_table;
pub use bridge::{
    query_bridge_fdb, query_bridge_port_mapping, query_port_vlan_membership, query_vlan_table,
};
pub use cdp::query_cdp_neighbors;
pub use entity::query_entity_physical;
pub use lldp::{
    CLASSIC_LLDP_MIB, LldpMibProfile, V2_LLDP_MIB, query_lldp_local, query_lldp_local_ports,
    query_lldp_neighbors, query_lldp_neighbors_for,
};
pub use system::{query_ip_addr_table, query_system_info, walk_if_table};
pub use walk::AgentErrorStatus;
use walk::{walk_column, walk_subtree};

/// Varbinds requested per getbulk round when walking a table subtree.
const BULK_MAX_REPETITIONS: u32 = 20;

/// A single `getbulk` round-trip's non-error outcome. Transport failures (timeouts,
/// session errors) are the `Err` arm of the returned `Result`; the legitimate non-error
/// signals are an agent that refuses getbulk, which the walk retries via getnext, and one
/// that refuses the page it was asked for, which the walk retries smaller.
/// Varbinds borrow the session's response buffer (`snmp2::Value<'a>` holds `&'a [u8]`
/// for octet strings), so a page is only valid while the session stays borrowed.
pub type Varbinds<'a> = Vec<(Vec<u64>, Value<'a>)>;

/// SNMP `noSuchName(2)`. SNMPv1's only way to say a GETNEXT has run off the end of the MIB view;
/// RFC 3584 §4.2.2.2.2 maps v2's `endOfMibView` to exactly this for a v1 client.
pub const SNMP_ERR_NO_SUCH_NAME: u32 = 2;

pub enum WalkPage<'a> {
    /// Decoded varbinds in wire order, OIDs as sub-id vectors.
    Varbinds(Varbinds<'a>),
    /// Agent rejected getbulk (e.g. SNMPv1) — retry from the same OID with getnext.
    BulkUnsupported,
    /// Agent answered with a non-zero `error-status`, which the walk takes as a refusal of the
    /// page size and retries from the same OID with fewer repetitions.
    ///
    /// `tooBig` is the status RFC 3416 names for this, sent with an empty varbind list. GH #710's
    /// Hikvision sends `genErr` instead, with the request's varbind echoed, and serves the same
    /// column at ten repetitions that it refuses at twenty.
    Refused { error_status: u32 },
}

impl<'a> WalkPage<'a> {
    /// A getbulk response as the walk reads it. Production and the simulator both come through
    /// here, so a device test exercises the same reading of the PDU a live session does.
    ///
    /// Any non-zero status is a refusal and none of its varbinds is a row: an agent refusing a
    /// request echoes the request's varbinds back (RFC 3416 §4.2). snmp2's `Pdu::validate` checks
    /// type, request id and community and never the status, so this is the one place a refused
    /// page is told from a real one. Read as rows, the Hikvision's echo of the column base was an
    /// OID that neither sits in the subtree nor advances, which the staleness guard rightly took
    /// for an answer to another request, and every table truncated with nothing.
    pub fn from_bulk_response(error_status: u32, varbinds: Varbinds<'a>) -> Self {
        if error_status == 0 {
            Self::Varbinds(varbinds)
        } else {
            Self::Refused { error_status }
        }
    }
}

/// A getnext or get response as its callers read it. Shared by production and the simulator for
/// the same reason as [`WalkPage::from_bulk_response`].
///
/// A non-zero status never hands its echoed varbinds back. `noSuchName` becomes `endOfMibView` at
/// the requested OID, the v2 exception every caller already treats as "nothing more here"; any
/// other status is an [`AgentErrorStatus`], since neither caller has a smaller question to ask.
pub fn response_varbinds<'a>(
    requested: &[u64],
    error_status: u32,
    varbinds: Varbinds<'a>,
) -> Result<Varbinds<'a>> {
    match error_status {
        0 => Ok(varbinds),
        SNMP_ERR_NO_SUCH_NAME => Ok(vec![(requested.to_vec(), Value::EndOfMibView)]),
        status => Err(AgentErrorStatus(status).into()),
    }
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

    /// Whether this session has already established that the device will not serve getbulk.
    ///
    /// Scoped to the session, which lives exactly as long as one host's collection, so the answer
    /// is per-device and re-learned on the next scan. `walk_subtree` seeds itself from this and
    /// sets it through [`Self::note_getbulk_unusable`] when it falls back, so a device costs the
    /// discovery one column's worth of timeouts rather than one per column: GH #668's switch3
    /// spent fifteen seconds finding this out on `lldpRemChassisId` and fifteen more on
    /// `lldpRemManAddrIfSubtype` immediately afterwards.
    ///
    /// Defaulted rather than required because the answer only matters to a transport that outlives
    /// a single walk. The fakes that serve one column apiece are right to say `false`.
    fn getbulk_unusable(&self) -> bool {
        false
    }

    /// Record that getbulk did not work on this device.
    fn note_getbulk_unusable(&mut self) {}

    /// Whether the discovery this session serves has been cancelled. `walk_subtree` checks it
    /// before every request. Defaulted for the fakes, which serve one column and never cancel.
    fn cancelled(&self) -> bool {
        false
    }

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
impl SnmpWalkTransport for super::session::SnmpSession {
    fn getbulk_unusable(&self) -> bool {
        Self::getbulk_unusable(self)
    }

    fn note_getbulk_unusable(&mut self) {
        Self::note_getbulk_unusable(self);
    }

    fn cancelled(&self) -> bool {
        self.is_cancelled()
    }

    async fn walk_getbulk<'a>(
        &'a mut self,
        from: &[u64],
        max_repetitions: u32,
    ) -> Result<WalkPage<'a>> {
        let oid = Oid::from(from).map_err(|_| anyhow::anyhow!("invalid walk OID"))?;
        match timeout(SNMP_TIMEOUT, self.getbulk(&[&oid], 0, max_repetitions)).await {
            Ok(Ok(pdu)) => Ok(WalkPage::from_bulk_response(
                pdu.error_status,
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
            Ok(Ok(pdu)) => response_varbinds(
                from,
                pdu.error_status,
                pdu.varbinds.map(|(o, v)| (oid_to_vec(&o), v)).collect(),
            ),
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
            Ok(Ok(response)) => Ok(response_varbinds(
                oid,
                response.error_status,
                response
                    .varbinds
                    .map(|(o, v)| (oid_to_vec(&o), v))
                    .collect(),
            )?
            .into_iter()
            .next()
            .filter(|(resp, _)| resp.as_slice() == oid)
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
    /// Agent kept answering getnext with an error status. The detail names the status.
    ErrorStatus,
    /// The discovery was cancelled partway through the walk. Rows already read stay in the
    /// collector, and the column counts as cut short, so a truncated table is never taken as the
    /// whole of it (the GH #649 prune).
    Cancelled,
}

impl From<WalkStop> for Option<ShortfallReason> {
    /// Collapse the walk's own vocabulary into the four things an operator can act on
    /// differently. The distinctions dropped here (`EmptyResponse`, `ErrorStatus` and
    /// `Transport`) are diagnostic detail, already in the truncation log with the host address.
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
            WalkStop::Transport | WalkStop::EmptyResponse | WalkStop::ErrorStatus => {
                Some(ShortfallReason::NoAnswer)
            }
            // Nothing about the device to report: the run itself was stopped.
            WalkStop::Cancelled => None,
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

#[cfg(test)]
#[path = "tests/walk_tests.rs"]
mod walk_tests;

/// `walk_if_table` assembles one interface per ifIndex across eleven separate column walks, and
/// until now had no test at all — the multi-column assembly, the row-minting and the `complete`
/// aggregation were all uncovered, which is how a foreign interface ended up on a switch and was
/// still reported as an authoritative full ifTable.
#[cfg(test)]
#[path = "tests/if_table_tests.rs"]
mod if_table_tests;

/// GH #674: an agent whose table rows are not in ascending OID order.
///
/// Firmware that stores a table unsorted and iterates it positionally answers GETNEXT with
/// whatever row comes next *in its own order*. That is what makes `snmpwalk` stop with "OID not
/// increasing" while `snmpbulkwalk -Cc` reads the same table in full: the rows are real and
/// retrievable, and only a client that insists every step ascend refuses them.
#[cfg(test)]
#[path = "tests/out_of_order_tests.rs"]
mod out_of_order_tests;

#[cfg(test)]
#[path = "tests/lldp_v2_tests.rs"]
mod lldp_v2_tests;
