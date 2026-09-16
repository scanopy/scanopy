//! Typed scan findings, accumulated during a run.
//!
//! Producers that fire per host record a typed value here rather than pushing a sentence, and the
//! session turns them into [`DiscoveryWarning`]s at finalize. Two rules hold for every record
//! below:
//!
//! - **Say what it means for the user, not what the code saw.** The code an operator ends up
//!   reading has to name a consequence they can act on; the internal completeness flag is not one.
//! - **Never truncate silently.** Whatever is dropped says how much was dropped, because a list
//!   that simply stops reads as "that was all of them".
//!
//! The English itself lives in [`DiscoveryWarningCode`]'s metadata, not here. What this module
//! decides is *which* code each record is an instance of — the discriminating part of the record
//! becomes the code, and everything else rides along as that occurrence's own detail.

mod coding;

pub use coding::{
    warn_contradicted_claims, warn_credential_issues, warn_equal_reach_integrations,
    warn_incomplete_interface_walks, warn_incomplete_snmp_walks, warn_malformed_neighbours,
    warn_snmp_collected_nothing, warn_unresolved_lldp_ports, warn_vlan_recording_failures,
};

use std::net::IpAddr;
use uuid::Uuid;

// Re-exported so the SNMP integration keeps building records against one path, even though the
// two value enums are wire types now and live with the codes.
pub use crate::daemon::discovery::types::warnings::{ClaimSource, SnmpWalkGroup};

use crate::server::credentials::r#impl::mapping::{
    CredentialQueryPayloadDiscriminants, is_unresolvable_credential,
};
use crate::server::ports::r#impl::base::PortType;

// ============================================================================
// Incomplete SNMP walks
// ============================================================================

/// Why a group of SNMP data came up short.
///
/// The renderer used to guess — its empty-walk line said the query "usually timed out" — because
/// the reason stopped at the walk and never travelled with the result. Each of these calls for a
/// different sentence, and two of them are not faults at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShortfallReason {
    /// Stopped at our own entry cap. The device is fine and larger than we read.
    ///
    /// Carries the limit so the renderer can name it without reaching into the SNMP module for
    /// a constant — the number is the whole point of the sentence.
    EntryCap { limit: usize },
    /// The agent stopped answering partway.
    NoAnswer,
    /// The agent answered out of step with what was asked — a stale or non-advancing response.
    Desynchronised,
    /// The agent does not implement this MIB. Not a fault, and no later scan will change it.
    Unsupported,
}

impl ShortfallReason {
    /// The read was cut off partway rather than finishing on a table this size.
    ///
    /// Distinguishes a device that answered everything asked of it from one that stopped
    /// answering. Only the first can be said to have declined to serve rows or to have
    /// miscounted itself; the second simply did not get that far, and a line claiming otherwise
    /// contradicts the shortfall line reporting the same device.
    fn read_was_cut_short(self) -> bool {
        matches!(self, Self::NoAnswer | Self::Desynchronised)
    }
}

/// What a device led us to expect, so a collection can report being short-changed.
///
/// The reporting surface is otherwise entirely self-referential: it can say a walk did not finish,
/// that we hit our own cap, or that rows were discarded, but never that the device itself said
/// there was more. These are the inputs that make the second kind of statement possible, and both
/// were already declared in `oids.rs` and never read.
///
/// A claim is evidence, never a verdict. Devices misreport their own counts, so a contradiction
/// is a warning and the scan keeps everything it read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceClaim {
    /// The device published a row count for this group.
    Count {
        source: ClaimSource,
        expected: usize,
    },
    /// The device declared the capability without publishing a count, so the only contradiction
    /// it can produce is having read nothing at all.
    Implements { source: ClaimSource },
}

/// LLDP neighbours whose local port could not be placed on an interface of the device.
///
/// Kept apart from [`IncompleteSnmpWalk`] because it is a different kind of problem and reads
/// nothing like one: the walk succeeded and the neighbours are there, but their `lldpLocPortNum`
/// could not be translated to an `ifIndex`.
///
/// Two outcomes follow, and the difference is what the operator needs. Where the untranslated
/// number happens to name a real interface, the neighbour attaches to it and the map is wrong in a
/// specific place. Where it names none, the neighbour is attached nowhere and discarded whole — no
/// chassis id is stored, no link is drawn, and the device reads as though it had no LLDP data at
/// all. The second is the common case on a device whose `lldpLocPortTable` is absent or unreadable,
/// and it was the outcome this warning previously described as "may be drawn against the wrong
/// port".
/// Whether a placement failure is attributable to the device's numbering or to our read of it.
///
/// The two call for opposite things and were reported as one sentence offering both, which left
/// the operator to guess (GH #668). A device that numbers its LLDP ports in a space of its own has
/// told us everything it has and no rescan changes it; a device whose port table we only half read
/// may well place its neighbours next time.
///
/// Derived from the reads rather than stored per neighbour: it is a property of what the walk got,
/// not of any one row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalPortPlacementReason {
    /// Both `lldpLocPortTable` and the interface set were read to their end, so what did not line
    /// up genuinely does not line up.
    ReadsComplete,
    /// One of them stopped early, so part of the numbering was never read.
    ReadCutShort,
}

impl LocalPortPlacementReason {
    /// Which of the two this is, from the completeness of the reads the placement used.
    ///
    /// Both tables, because either one being short costs a placement: the port table names the
    /// port and the interface set is what a name is matched against, and a neighbour cannot be
    /// placed if either half is missing. `if_set_complete` rather than the attribute columns —
    /// pruning and placement both turn on the *set*, and a blank `ifAlias` costs neither.
    pub fn from_reads(local_ports_complete: bool, if_set_complete: bool) -> Self {
        if local_ports_complete && if_set_complete {
            Self::ReadsComplete
        } else {
            Self::ReadCutShort
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedLldpPorts {
    pub ip: IpAddr,
    /// Neighbours no tier could place, which keep their raw `lldpLocPortNum`.
    pub unresolved: usize,
    /// Neighbours that reached no interface and were discarded. A subset in effect, not in
    /// arithmetic: a neighbour can be placed and still be dropped for colliding with another on
    /// the same port, and one can be unplaced and still land on a real ifIndex.
    pub dropped: usize,
    /// How many neighbours the device reported in total, so a figure has a denominator.
    pub total: usize,
    /// Whether this is the device's numbering or our read of it. Decides which of the two
    /// warnings the operator gets, because the sentence covering both was wrong half the time.
    pub reason: LocalPortPlacementReason,
}

/// A device that answered the credential and then produced nothing at all.
///
/// Every other warning here describes one group falling short of the rest. This one is for the
/// case where there is no rest: the probe succeeded, so the address, the port and the community
/// or USM user are all right, and then every table came back empty. GH #674 is what that looks
/// like — a switch logged at INFO with `count=0` five times over, reported to the operator as a
/// clean scan, and diagnosable only by someone reading the daemon log line by line.
///
/// Kept separate from [`IncompleteSnmpWalk`] rather than emitted as one line per empty group,
/// because a device that read nothing has not failed eight ways. It is also not necessarily a
/// fault of ours — a camera switch may genuinely implement nothing past `system` — so the line
/// says what was observed and what to check, and does not promise a rescan will help.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnmpCollectedNothing {
    pub ip: IpAddr,
}

/// Neighbour records a device served without the identifier that makes them usable.
///
/// A third kind of problem again, and it must not read like either neighbour above. The walk
/// finished and the rows arrived; they are missing a mandatory field — the LLDP chassis ID
/// (IEEE 802.1AB) or the CDP device id — which is what L2 resolution matches the far end on, so
/// they are discarded rather than written over good data.
///
/// The distinction that matters to an operator: **no rescan will fix this**. "Incomplete" invites
/// a retry, and a retry against a switch whose firmware serves malformed records produces exactly
/// the same result. It reached us as a bare `dropped=N` in the daemon log, which is how a customer
/// ended up hand-running snmpwalk to tell us what their switch had sent (GH #668).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedNeighbours {
    pub ip: IpAddr,
    pub group: SnmpWalkGroup,
    pub discarded: usize,
    /// Records that survived, so the line can say whether this cost the device some of its
    /// topology or all of it.
    pub kept: usize,
    /// What accounts for most of the loss on this device.
    pub reason: MalformedNeighbourReason,
}

/// Why a device's neighbour records could not be used.
///
/// The one thing an operator can act on here is a rescan, and exactly one of these four is worth
/// spending it on. The single line this replaced told all of them the same thing — that the device
/// answered in full and a retry would change nothing — which is right for three and wrong for the
/// fourth, the case where a retry is the whole remedy.
///
/// Ordered as declared: renderers group by this, so the ordering fixes the order of the lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MalformedNeighbourReason {
    /// The column carrying the identifier stopped before the columns that follow it did, so the
    /// rows arrived describing a neighbour they could not name.
    ///
    /// The only cause here a rescan can fix, and it overrides the rest: a row a truncated column
    /// never reached is indistinguishable from one it never had, so once truncation is in
    /// evidence the other classifications below cannot be trusted for the same device.
    WalkCutShort,
    /// Rows appeared in the descriptive columns at table positions the identifying column never
    /// listed — there was never an identifier on them to lose.
    GhostRows,
    /// The identifying column listed the row and then never supplied a usable value for it. The
    /// read finished, so the gap is in what the device served.
    IncompleteRecords,
    /// The agent answered an identifying column with a value of a type that column cannot hold.
    UnexpectedType,
    /// The row's position in the device's table could not be read, so it could not be tied to a
    /// local port. A firmware serving an index shape the MIB does not describe (GH #668).
    UnreadableIndex,
}

/// A device whose VLAN table was read and could not be recorded.
///
/// Not a shortfall in a walk, and it must not read like one: the switch answered in full, and the
/// upsert to the server failed. Reporting it as "VLAN membership was incomplete" would send an
/// operator to inspect a switch that did nothing wrong, when the fault is on our side of the
/// wire. The consequence is real either way — every interface on that device loses its VLAN ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VlanRecordingFailed {
    pub ip: IpAddr,
}

/// A host whose interfaces two full-ifTable integrations both collected in one scan.
///
/// Not a fault: running SNMP and gNMI against one switch is supported, and every row either one
/// offered is kept. What the operator gets from this is where a port's row came from. On a port
/// both describe, `first` supplies the row and its neighbour candidates, and `second`'s copy is
/// dropped, so a link only `second` saw on that port is not drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EqualReachIntegrations {
    pub ip: IpAddr,
    /// The integration that contributed first, and so won every port both describe.
    pub first: CredentialQueryPayloadDiscriminants,
    pub second: CredentialQueryPayloadDiscriminants,
}

/// One SNMP data group that a walk could not read in full, for one device.
///
/// `returned_any` distinguishes the two cases the old single phrasing conflated. A walk that
/// returns rows and stops was genuinely truncated; a walk that returns nothing timed out or
/// errored outright — `Default` for these result types is `complete: false` with no records, so
/// "the device stopped responding partway through" was simply wrong for the second case and sent
/// operators to inspect hardware that was fine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncompleteSnmpWalk {
    pub ip: IpAddr,
    pub group: SnmpWalkGroup,
    pub returned_any: bool,
    /// Why it came up short, when the walk could establish that.
    pub reason: Option<ShortfallReason>,
}

/// What one host's SNMP collection managed to read, per group.
///
/// `complete` mirrors [`SnmpCollection::complete`] on the daemon side; `observed` is how many
/// records the group produced, and `claim` is what the device said to expect before we read it.
#[derive(Debug, Clone, Copy, Default)]
pub struct SnmpGroupOutcome {
    pub complete: bool,
    /// How many records this group actually produced.
    ///
    /// A count rather than the "did it return anything" flag it replaced, because the flag could
    /// only ever support the second half of a comparison the device had already made. Both
    /// consumers now read from one field, so they cannot drift.
    pub observed: usize,
    pub reason: Option<ShortfallReason>,
    /// What the device led us to expect here, when it said anything at all.
    pub claim: Option<DeviceClaim>,
}

impl SnmpGroupOutcome {
    /// Whether the group produced anything, which is all the shortfall prose needs to know.
    fn returned_any(&self) -> bool {
        self.observed > 0
    }

    /// The *read* fell short — as opposed to the read finishing and some of its rows being
    /// discarded as unusable.
    ///
    /// `complete` answers one question: may this result overwrite what the server holds? For the
    /// neighbour groups, two very different things clear it. A truncated walk clears it and
    /// records why, in `reason`. Discarding a malformed record also clears it — the rows that
    /// survived are not the whole picture — but the walk itself ran to its end, so there is no
    /// `reason` to record.
    ///
    /// Reporting the second as a short read put two lines on one device giving opposite advice:
    /// one promising the values would "refresh on the next complete scan", the other saying no
    /// rescan would ever change them (GH #668). The malformed-record warning already covers that
    /// case and states the true one.
    ///
    /// Scoped to the groups that discard. Elsewhere an absent `reason` carries its own meaning —
    /// a device with no bridge MIB answers nothing and has nothing to say about why — and must
    /// still be reported.
    fn walk_fell_short(&self, group: SnmpWalkGroup) -> bool {
        !(self.complete || (group.discards_malformed_records() && self.reason.is_none()))
    }
}

/// Per-group outcomes for one host, as [`snmp_walk_shortfalls`] consumes them.
#[derive(Debug, Clone, Copy, Default)]
pub struct SnmpCollectionOutcome {
    pub lldp: SnmpGroupOutcome,
    pub cdp: SnmpGroupOutcome,
    /// Present so the interface count can be checked against the device's own `ifNumber`.
    ///
    /// Deliberately absent from [`snmp_walk_shortfalls`]'s array: a short interface walk is
    /// reported by [`IncompleteInterfaceWalk`], and adding it here too would tell one operator
    /// the same thing twice in two vocabularies. See [`SnmpWalkGroup::Interfaces`].
    pub interfaces: SnmpGroupOutcome,
    pub bridge_port_numbering: SnmpGroupOutcome,
    pub bridge_forwarding: SnmpGroupOutcome,
    pub vlan_membership: SnmpGroupOutcome,
    pub arp_table: SnmpGroupOutcome,
    pub device_inventory: SnmpGroupOutcome,
    pub ip_addresses: SnmpGroupOutcome,
    pub lldp_local_ports: SnmpGroupOutcome,
    pub vlan_names: SnmpGroupOutcome,
}

/// The groups worth reporting for one host.
///
/// Bridge forwarding and VLAN membership are both keyed by `dot1dBasePortIfIndex`, so when *that*
/// walk fails they are marked incomplete having attempted nothing of their own. Reporting all
/// three told an operator their switch had failed three ways when it had failed once, and pointed
/// at the two tables that are consequences rather than the one that is the cause. Report the
/// cause, and stay silent about the derived groups unless they failed on their own account.
pub fn snmp_walk_shortfalls(ip: IpAddr, outcome: SnmpCollectionOutcome) -> Vec<IncompleteSnmpWalk> {
    let root_failed = !outcome.bridge_port_numbering.complete;
    [
        (SnmpWalkGroup::Lldp, outcome.lldp, true),
        (SnmpWalkGroup::Cdp, outcome.cdp, true),
        (
            SnmpWalkGroup::BridgePortNumbering,
            outcome.bridge_port_numbering,
            true,
        ),
        (
            SnmpWalkGroup::BridgeForwarding,
            outcome.bridge_forwarding,
            !root_failed,
        ),
        (
            SnmpWalkGroup::VlanMembership,
            outcome.vlan_membership,
            !root_failed,
        ),
        (SnmpWalkGroup::ArpTable, outcome.arp_table, true),
        (
            SnmpWalkGroup::DeviceInventory,
            outcome.device_inventory,
            true,
        ),
        (SnmpWalkGroup::IpAddresses, outcome.ip_addresses, true),
        (
            SnmpWalkGroup::LldpLocalPorts,
            outcome.lldp_local_ports,
            true,
        ),
        (SnmpWalkGroup::VlanNames, outcome.vlan_names, true),
    ]
    .into_iter()
    .filter(|(group, outcome, report)| *report && outcome.walk_fell_short(*group))
    .map(|(group, outcome, _)| IncompleteSnmpWalk {
        ip,
        group,
        returned_any: outcome.returned_any(),
        reason: outcome.reason,
    })
    .collect()
}

// ============================================================================
// Contradicted device claims
// ============================================================================

/// A device published a figure about itself and the collection read something else.
///
/// Kept apart from [`IncompleteSnmpWalk`] because the two state different facts and a device can
/// warrant both at once: a shortfall says why *we* stopped reading, a contradiction says what the
/// *device* said was there. #685 is exactly the pair — a bare "did not finish reporting LLDP
/// neighbours" on a switch that answers with an LLDP chassis ID of its own, where the shortfall
/// alone reads as a transient and the contradiction is what makes it worth chasing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContradictedClaim {
    pub ip: IpAddr,
    pub group: SnmpWalkGroup,
    pub claim: DeviceClaim,
    /// What the collection actually read.
    pub observed: usize,
    /// Why the read came up short, when it did.
    ///
    /// Carried so the line can stop short of naming a cause the shortfall line has already
    /// named differently. A device whose walk was cut off has not declined to serve anything and
    /// is not misreporting itself, and saying so beside a line that says it stopped responding
    /// leaves the reader to reconcile two accounts of one device.
    pub reason: Option<ShortfallReason>,
}

/// How far below its own claim a device has to land before it is worth a line.
///
/// Not strict inequality. Devices legitimately miscount by a small margin — an interface appearing
/// or going away between the scalar and the walk, a count that includes something the table does
/// not — and a warning that fires on a device off by one is a warning operators learn to skip.
/// Half is far enough below any rounding explanation to mean something went wrong, and the two
/// figures are always named so nobody has to take the threshold on trust.
const CLAIM_SHORTFALL_RATIO: usize = 2;

/// The claims this host contradicted, if any.
///
/// A pure function of the outcome so the policy is testable without a live agent, matching
/// [`snmp_walk_shortfalls`] next door.
///
/// Deliberately independent of both `complete` and `unsupported`. A contradiction is a fact about
/// the device's own statements, and suppressing it whenever the walk fell short would silence it
/// in precisely the cases it exists for: #685's device reports a shortfall, and a switch whose
/// bridge MIB answers `noSuchObject` while its `sysServices` bridge bit is set currently renders
/// as the benign "does not implement SNMP bridge-port numbering" rather than as a device
/// disagreeing with itself.
pub fn contradicted_claims(ip: IpAddr, outcome: SnmpCollectionOutcome) -> Vec<ContradictedClaim> {
    [
        (SnmpWalkGroup::Interfaces, outcome.interfaces),
        (SnmpWalkGroup::Lldp, outcome.lldp),
        (
            SnmpWalkGroup::BridgePortNumbering,
            outcome.bridge_port_numbering,
        ),
    ]
    .into_iter()
    .filter_map(|(group, group_outcome)| {
        let claim = group_outcome.claim?;

        // Our own cap is not the device's claim. A switch with more MAC entries than one scan
        // reads has not contradicted anything, and `EntryCap` already says so in its own line.
        if matches!(group_outcome.reason, Some(ShortfallReason::EntryCap { .. })) {
            return None;
        }

        let contradicted = match claim {
            DeviceClaim::Count { expected, .. } => {
                group_outcome.observed * CLAIM_SHORTFALL_RATIO <= expected
                    && group_outcome.observed < expected
            }
            DeviceClaim::Implements { .. } => !group_outcome.returned_any(),
        };

        contradicted.then_some(ContradictedClaim {
            ip,
            group,
            claim,
            observed: group_outcome.observed,
            reason: group_outcome.reason,
        })
    })
    .collect()
}

// ============================================================================
// Incomplete interface (ifTable) walks
// ============================================================================

/// One device whose ifTable walk fell short, and in which of the two ways.
///
/// Kept separate from [`IncompleteSnmpWalk`] because the two failures mean different things to
/// an operator and must not be merged into one sentence: a truncated interface *set* means
/// interfaces are genuinely missing, while a truncated attribute column only means some
/// descriptions or speeds are blank. Reporting the second as possible data loss sends people
/// hunting for interfaces that were never absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncompleteInterfaceWalk {
    pub ip: IpAddr,
    /// Interfaces read before the walk stopped.
    pub collected: usize,
    /// `true` when the whole interface set was read and only attribute columns fell short.
    pub set_complete: bool,
}

// ============================================================================
// Credential issues
// ============================================================================

/// How a credential attempt that actually ran came out.
///
/// Deliberately a *subset* of [`CredentialIssueReason`]: an integration observes only what
/// happened on the wire, and has no way to know about the dispatch-level reasons ("never
/// scanned", "gate closed"). Splitting the two means an integration cannot report a reason it
/// cannot have established.
///
/// Every variant maps to a different thing for the operator to do, which is the test for whether
/// one belongs here. "The device refused my password" and "nothing was listening" arrive as the
/// same empty result and send an operator to opposite ends of the problem.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    strum_macros::EnumCount,
    strum_macros::EnumIter,
)]
pub enum AttemptOutcome {
    /// The endpoint answered and refused the credential. The credential is wrong.
    Rejected,
    /// Nothing was listening: connection refused, or no route.
    Unreachable,
    /// Connected, then never answered inside the timeout.
    TimedOut,
    /// Something answered, but it is not this service — usually the wrong port.
    NotThisService,
    /// TLS negotiation failed. Distinct from [`Self::Rejected`] because the fix is a trust
    /// setting, not a password.
    TlsFailed,
    /// The credential itself is incomplete or unusable — our configuration rather than their
    /// device, and the only outcome an operator fixes in Scanopy rather than on the network.
    Malformed,
    /// The credential worked and the collection after it did not. The host's data is missing
    /// rather than merely stale, which is why it is worth its own line.
    CollectionFailed,
    /// The credential worked, the collection started, and the time limit stopped it before it
    /// finished.
    ///
    /// Distinct from [`Self::TimedOut`], which is the address never answering, and from
    /// [`Self::CollectionFailed`], which is the collection erroring: here the service answered
    /// perfectly well and there was simply more to read than there was time to read it. Reporting
    /// this as [`Self::TimedOut`] told operators to check that the service was listening, on a
    /// host where it had just enumerated 77 containers (GH #650).
    CollectionTimedOut,
    /// The scan was cancelled. Never rendered — the user stopped it, so it is not a finding.
    Cancelled,
}

impl AttemptOutcome {
    /// Reclassify a credential error as a configuration fault when the credential itself could not
    /// be read.
    ///
    /// Applied at each integration's probe boundary rather than at each `resolve` call, so an
    /// integration gets the right verdict without having to remember to ask for it — the answer
    /// depends only on the error, and the error already carries it. `otherwise` is whatever that
    /// integration would have concluded, which stays correct for every failure that did reach the
    /// network.
    ///
    /// The distinction is not cosmetic. [`Self::Unreachable`] and [`Self::Rejected`] both send an
    /// operator to the device — check it is online, check the password — and neither is where the
    /// problem is when the daemon never managed to read the credential at all (GH #668).
    pub fn for_credential_error(error: &anyhow::Error, otherwise: Self) -> Self {
        if is_unresolvable_credential(error) {
            Self::Malformed
        } else {
            otherwise
        }
    }
}

/// Why an IP-targeted credential produced nothing.
///
/// Only credentials the user deliberately assigned to a host are reported. A network-default
/// credential failing is routine — it is broadcast at every address in the subnet — and
/// reporting those would flood the notification on any sweep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialIssueReason {
    /// The address is not inside any subnet this scan enumerated, so it was never contacted.
    TargetNotScanned,
    /// The address *was* in scope, but nothing answered there, so no host was ever deep-scanned
    /// and the credential was never applied. Distinct from [`Self::TargetNotScanned`] because
    /// the fix is different: there the discovery's subnets are wrong, here the address is wrong
    /// or the host is down.
    TargetNotResponding,
    /// The host was scanned but the credential's port was not open, so no probe was attempted.
    GateClosed { ports: Vec<PortType> },
    /// The attempt ran and did not succeed.
    ///
    /// Replaces a `ProbeRejected` variant that named one outcome and was used for all of them —
    /// every integration flattened cancellation, a closed port, a TLS failure and a genuinely
    /// wrong password into the same "was rejected" line.
    Attempted {
        outcome: AttemptOutcome,
        /// The integration's own diagnostic, already phrased for an operator.
        message: String,
    },
}

/// One IP-targeted credential that did not work, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialIssue {
    /// Which integration's credential this was.
    ///
    /// The discriminant rather than `CredentialQueryPayload::discovery_label`'s English: this is
    /// what labels the warning metric and what the UI resolves a translated credential-type name
    /// from, and a display string can do neither.
    pub integration: CredentialQueryPayloadDiscriminants,
    pub ip: IpAddr,
    pub reason: CredentialIssueReason,
    /// The stored credential behind the issue, where the dispatch that found it had one.
    ///
    /// Carried on the issue rather than looked up later because this is the only point at which
    /// the credential that was actually tried is still identified — by the time the warning is
    /// read back, address and integration are all that remain, and neither picks one record out
    /// of a network with two SNMP communities.
    pub credential_id: Option<Uuid>,
}

/// Whether a finished credential attempt is worth telling the operator about, and as what.
///
/// Pure, and separated from the dispatch that calls it for the same reason
/// [`snmp_walk_shortfalls`] is: the policy is the part worth testing, and testing it in place
/// would mean standing up a whole `DaemonDiscoveryService`. Both the probe path and the execute
/// path go through here, so the two cannot drift apart.
///
/// Two things are deliberately silent:
///
/// - **A network default that failed.** It is broadcast at every address in the subnet, so
///   failing is its normal condition — reporting it would bury the real findings under a line per
///   unresponsive host on any /24 sweep. Only a credential the user pinned to this host is news.
/// - **A cancelled attempt.** The operator stopped the scan; nothing about that is a finding, and
///   it used to be reported as a rejected credential.
pub fn issue_for_attempt(
    integration: CredentialQueryPayloadDiscriminants,
    ip: IpAddr,
    outcome: AttemptOutcome,
    message: String,
    user_assigned: bool,
    credential_id: Option<Uuid>,
) -> Option<CredentialIssue> {
    if outcome == AttemptOutcome::Cancelled {
        return None;
    }
    // The network-default rule is about a credential that reached a device and did not work. A
    // malformed one reached nothing: it is wrong in our own configuration, wrong identically at
    // every address, and stays wrong until someone re-enters it — so being a default is no reason
    // to keep quiet, and being quiet is what left GH #668's reporter with a scan that said nothing
    // at all. `warn_credential_issues` collapses these to one finding so this cannot flood.
    if !user_assigned && outcome != AttemptOutcome::Malformed {
        return None;
    }
    Some(CredentialIssue {
        integration,
        ip,
        reason: CredentialIssueReason::Attempted { outcome, message },
        credential_id,
    })
}
#[cfg(test)]
mod tests {
    use super::*;

    /// The groups a shortfall test is not varying, read cleanly so they report nothing.
    ///
    /// Not `SnmpCollectionOutcome::default()`: a defaulted `SnmpGroupOutcome` is a walk that ran
    /// and failed, so spreading it would add a finding per group these tests never mention.
    fn quiet_groups() -> SnmpCollectionOutcome {
        let clean = SnmpGroupOutcome {
            complete: true,
            observed: 1,
            reason: None,
            claim: None,
        };
        SnmpCollectionOutcome {
            lldp: clean,
            cdp: clean,
            interfaces: clean,
            bridge_port_numbering: clean,
            bridge_forwarding: clean,
            vlan_membership: clean,
            arp_table: clean,
            device_inventory: clean,
            ip_addresses: clean,
            lldp_local_ports: clean,
            vlan_names: clean,
        }
    }

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    /// A short interface walk is reported by `IncompleteInterfaceWalk`, in its own vocabulary.
    /// `SnmpWalkGroup::Interfaces` exists so the count can be checked against `ifNumber`, and
    /// telling one operator their interfaces came up short twice, in two phrasings, is what the
    /// separation prevents.
    #[test]
    fn the_interfaces_group_is_never_reported_as_a_short_walk() {
        let outcome = SnmpCollectionOutcome {
            interfaces: SnmpGroupOutcome {
                complete: false,
                observed: 0,
                reason: Some(ShortfallReason::NoAnswer),
                claim: None,
            },
            ..quiet_groups()
        };

        assert!(
            snmp_walk_shortfalls(ip("192.168.7.248"), outcome).is_empty(),
            "interface shortfalls belong to warn_incomplete_interface_walks"
        );
    }

    /// The suppression above is scoped to the groups keyed by `dot1dBasePortIfIndex`. The ARP
    /// table, hardware inventory and the rest are read from unrelated MIBs, so a switch with no
    /// bridge MIB says nothing about whether they were read — theirs is a finding of its own.
    ///
    /// Until GH #674 they could not be reported at all: those walks dropped their stop on the
    /// floor, so a truncated ARP table reached the operator as no data and no explanation.
    #[test]
    fn a_group_unrelated_to_the_bridge_mib_reports_even_when_the_bridge_mib_failed() {
        let shortfalls = snmp_walk_shortfalls(
            ip("192.168.7.246"),
            SnmpCollectionOutcome {
                bridge_port_numbering: SnmpGroupOutcome::default(),
                bridge_forwarding: SnmpGroupOutcome::default(),
                vlan_membership: SnmpGroupOutcome::default(),
                arp_table: SnmpGroupOutcome {
                    complete: false,
                    observed: 1,
                    claim: None,
                    reason: Some(ShortfallReason::Desynchronised),
                },
                ..quiet_groups()
            },
        );

        let groups: Vec<SnmpWalkGroup> = shortfalls.iter().map(|s| s.group).collect();
        assert_eq!(
            groups,
            vec![SnmpWalkGroup::BridgePortNumbering, SnmpWalkGroup::ArpTable]
        );
    }

    /// A switch with no bridge MIB fails three groups at once, but only one of them is a
    /// finding: the other two never ran. Reporting all three read as three separate faults.
    #[test]
    fn a_failed_bridge_port_walk_suppresses_the_groups_keyed_by_it() {
        let shortfalls = snmp_walk_shortfalls(
            ip("192.168.210.217"),
            SnmpCollectionOutcome {
                lldp: SnmpGroupOutcome {
                    complete: true,
                    observed: 0,
                    claim: None,
                    reason: None,
                },
                cdp: SnmpGroupOutcome {
                    complete: true,
                    observed: 0,
                    claim: None,
                    reason: None,
                },
                // Everything below is a consequence of this one failure.
                bridge_port_numbering: SnmpGroupOutcome::default(),
                bridge_forwarding: SnmpGroupOutcome::default(),
                vlan_membership: SnmpGroupOutcome::default(),
                ..quiet_groups()
            },
        );

        let groups: Vec<SnmpWalkGroup> = shortfalls.iter().map(|s| s.group).collect();
        assert_eq!(groups, vec![SnmpWalkGroup::BridgePortNumbering]);
    }

    /// A device that answered in full and had a malformed record thrown away is not a short read.
    ///
    /// Discarding clears the same `complete` flag a truncated walk does — it gates whether the
    /// result may overwrite the server's copy, and a thinned result may not. Reporting it here as
    /// well put two lines on one device telling the operator opposite things: that the values
    /// would "refresh on the next complete scan", and that no rescan would ever change them. The
    /// malformed-record warning covers this case on its own and states the true one.
    #[test]
    fn a_discarded_record_is_not_also_reported_as_a_short_walk() {
        let shortfalls = snmp_walk_shortfalls(
            ip("192.168.7.243"),
            SnmpCollectionOutcome {
                // What `query_lldp_neighbors` produces after discarding a ghost row: incomplete,
                // rows returned, and no walk-level reason because no walk stopped early.
                lldp: SnmpGroupOutcome {
                    complete: false,
                    observed: 1,
                    claim: None,
                    reason: None,
                },
                cdp: SnmpGroupOutcome {
                    complete: true,
                    observed: 0,
                    claim: None,
                    reason: None,
                },
                bridge_port_numbering: SnmpGroupOutcome {
                    complete: true,
                    observed: 1,
                    claim: None,
                    reason: None,
                },
                bridge_forwarding: SnmpGroupOutcome {
                    complete: true,
                    observed: 1,
                    claim: None,
                    reason: None,
                },
                vlan_membership: SnmpGroupOutcome {
                    complete: true,
                    observed: 1,
                    claim: None,
                    reason: None,
                },
                ..quiet_groups()
            },
        );

        assert!(
            shortfalls.is_empty(),
            "the walk finished; only the malformed-record warning should speak for this device, \
             and it says a rescan will not help — {shortfalls:?}"
        );
    }

    /// The converse, so the rule above cannot silence a real one: a walk that genuinely stopped
    /// early records why, and that is still the device's own finding.
    #[test]
    fn a_walk_that_stopped_early_is_still_reported() {
        let shortfalls = snmp_walk_shortfalls(
            ip("192.168.7.243"),
            SnmpCollectionOutcome {
                lldp: SnmpGroupOutcome {
                    complete: false,
                    observed: 1,
                    claim: None,
                    reason: Some(ShortfallReason::NoAnswer),
                },
                cdp: SnmpGroupOutcome {
                    complete: true,
                    observed: 0,
                    claim: None,
                    reason: None,
                },
                bridge_port_numbering: SnmpGroupOutcome {
                    complete: true,
                    observed: 1,
                    claim: None,
                    reason: None,
                },
                bridge_forwarding: SnmpGroupOutcome {
                    complete: true,
                    observed: 1,
                    claim: None,
                    reason: None,
                },
                vlan_membership: SnmpGroupOutcome {
                    complete: true,
                    observed: 1,
                    claim: None,
                    reason: None,
                },
                ..quiet_groups()
            },
        );

        let groups: Vec<SnmpWalkGroup> = shortfalls.iter().map(|s| s.group).collect();
        assert_eq!(groups, vec![SnmpWalkGroup::Lldp]);
    }

    /// The suppression is scoped to the root failing. When the bridge-port walk succeeds, a
    /// genuinely short FDB walk is the device's own finding and must still be reported.
    #[test]
    fn a_short_fdb_walk_is_reported_when_the_bridge_port_walk_succeeded() {
        let shortfalls = snmp_walk_shortfalls(
            ip("192.168.210.217"),
            SnmpCollectionOutcome {
                lldp: SnmpGroupOutcome {
                    complete: true,
                    observed: 1,
                    claim: None,
                    reason: None,
                },
                cdp: SnmpGroupOutcome {
                    complete: true,
                    observed: 0,
                    claim: None,
                    reason: None,
                },
                bridge_port_numbering: SnmpGroupOutcome {
                    complete: true,
                    observed: 1,
                    claim: None,
                    reason: None,
                },
                bridge_forwarding: SnmpGroupOutcome {
                    complete: false,
                    observed: 1,
                    claim: None,
                    reason: None,
                },
                vlan_membership: SnmpGroupOutcome {
                    complete: true,
                    observed: 1,
                    claim: None,
                    reason: None,
                },
                ..quiet_groups()
            },
        );

        let groups: Vec<SnmpWalkGroup> = shortfalls.iter().map(|s| s.group).collect();
        assert_eq!(groups, vec![SnmpWalkGroup::BridgeForwarding]);
        assert!(shortfalls[0].returned_any);
    }

    /// A device that miscounts itself by a little is not a device that failed, and warning about
    /// it teaches operators to skip the warning that matters. Sixteen of seventeen is the shape
    /// of an interface appearing between the scalar read and the walk.
    #[test]
    fn a_device_slightly_off_its_own_count_is_not_reported() {
        let outcome = SnmpCollectionOutcome {
            interfaces: SnmpGroupOutcome {
                complete: true,
                observed: 16,
                reason: None,
                claim: Some(DeviceClaim::Count {
                    source: ClaimSource::IfNumber,
                    expected: 17,
                }),
            },
            ..quiet_groups()
        };

        assert!(
            contradicted_claims(ip("192.168.7.247"), outcome).is_empty(),
            "a device one interface short of its own count is not a finding"
        );
    }

    /// Our own cap is not the device's claim. A switch with more entries than one scan reads has
    /// contradicted nothing, and `EntryCap` already says so in its own line — reporting both
    /// would put two explanations of the same number on one device, one of them wrong.
    #[test]
    fn our_own_entry_cap_is_not_a_contradiction() {
        let outcome = SnmpCollectionOutcome {
            interfaces: SnmpGroupOutcome {
                complete: false,
                observed: 10,
                reason: Some(ShortfallReason::EntryCap { limit: 10 }),
                claim: Some(DeviceClaim::Count {
                    source: ClaimSource::IfNumber,
                    expected: 4000,
                }),
            },
            ..quiet_groups()
        };

        assert!(
            contradicted_claims(ip("192.168.7.240"), outcome).is_empty(),
            "hitting our own cap is not the device disagreeing with itself"
        );
    }

    /// A device that says nothing about itself can contradict nothing. Most groups on most
    /// devices are this, so a claim-less outcome staying silent is what keeps the mechanism from
    /// being the noise it was built to replace.
    #[test]
    fn a_group_the_device_made_no_claim_about_is_never_reported() {
        let outcome = SnmpCollectionOutcome {
            lldp: SnmpGroupOutcome {
                complete: false,
                observed: 0,
                reason: Some(ShortfallReason::NoAnswer),
                claim: None,
            },
            ..quiet_groups()
        };

        assert!(contradicted_claims(ip("192.168.7.231"), outcome).is_empty());
    }

    /// A contradiction and a shortfall are different statements about one device, and it can
    /// warrant both: the shortfall says why the read stopped, the contradiction says what the
    /// device said was waiting. Neither may swallow the other.
    #[test]
    fn a_contradiction_does_not_replace_the_shortfall_line() {
        let outcome = SnmpCollectionOutcome {
            lldp: SnmpGroupOutcome {
                complete: false,
                observed: 0,
                reason: Some(ShortfallReason::NoAnswer),
                claim: Some(DeviceClaim::Implements {
                    source: ClaimSource::LldpLocalIdentity,
                }),
            },
            ..quiet_groups()
        };
        let addr = ip("192.168.200.151");

        assert_eq!(snmp_walk_shortfalls(addr, outcome).len(), 1);
        assert_eq!(contradicted_claims(addr, outcome).len(), 1);
    }

    /// The credential the fixture attempts under, so the tests can tell "carried through"
    /// from "happens to be None".
    fn tried_credential() -> Uuid {
        Uuid::from_u128(0x5ca0)
    }

    fn decide(outcome: AttemptOutcome, user_assigned: bool) -> Option<CredentialIssue> {
        issue_for_attempt(
            CredentialQueryPayloadDiscriminants::Snmp,
            ip("10.0.0.1"),
            outcome,
            "detail from the integration".to_string(),
            user_assigned,
            Some(tried_credential()),
        )
    }

    /// A network default is tried at every address in the subnet, so failing is its normal
    /// condition. Reporting it would put a line per unresponsive host into the notification and
    /// bury the credentials the user actually configured.
    #[test]
    fn a_failing_network_default_is_not_a_finding() {
        assert!(decide(AttemptOutcome::Rejected, false).is_none());
        assert!(decide(AttemptOutcome::TlsFailed, false).is_none());
    }

    /// GH #668: except when the credential could not be read at all.
    ///
    /// The network-default rule is about a credential that reached a device and did not work —
    /// broadcast at every address, so failing at most of them is its job. A credential we cannot
    /// resolve never reaches anything: it is broken in our own configuration, it is broken
    /// identically at every address, and it will stay broken until someone re-enters it. The
    /// reporter's community string was typed into a file-path field and the scan said nothing at
    /// all, on any host, for exactly this reason.
    #[test]
    fn a_credential_that_could_not_be_read_is_a_finding_even_as_a_network_default() {
        let issue = decide(AttemptOutcome::Malformed, false)
            .expect("a credential that cannot be read is a configuration fault, not a sweep miss");

        assert!(matches!(
            issue.reason,
            CredentialIssueReason::Attempted {
                outcome: AttemptOutcome::Malformed,
                ..
            }
        ));
    }

    /// The operator stopped the scan. This used to be reported as a rejected credential — the
    /// container probe returned the same failure for cancellation as for a refused socket.
    #[test]
    fn a_cancelled_attempt_is_not_a_finding_even_when_configured() {
        assert!(decide(AttemptOutcome::Cancelled, true).is_none());
    }

    /// The case the whole mechanism exists for: a credential the user pinned to this host was
    /// refused, and both the outcome and the integration's own wording survive to the renderer.
    #[test]
    fn a_configured_credential_that_was_refused_is_reported_in_full() {
        let issue = decide(AttemptOutcome::Rejected, true).expect("should be reported");
        assert_eq!(issue.ip, ip("10.0.0.1"));
        assert_eq!(issue.integration, CredentialQueryPayloadDiscriminants::Snmp);
        // Which record to go and fix — the address alone does not say, on a network with two
        // communities configured.
        assert_eq!(issue.credential_id, Some(tried_credential()));
        match issue.reason {
            CredentialIssueReason::Attempted { outcome, message } => {
                assert_eq!(outcome, AttemptOutcome::Rejected);
                assert_eq!(message, "detail from the integration");
            }
            other => panic!("expected an Attempted reason, got {other:?}"),
        }
    }

    /// `execute` only runs after a credential has already worked against this host, so dispatch
    /// passes `user_assigned: true` there unconditionally — there is no broadcast-noise case to
    /// suppress. This pins that a collection failure survives the decision.
    #[test]
    fn a_collection_failure_after_a_working_credential_is_reported() {
        assert!(decide(AttemptOutcome::CollectionFailed, true).is_some());
    }
}
