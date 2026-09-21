//! Coded discovery scan warnings.
//!
//! A warning used to be an English sentence built on the daemon and rendered verbatim into the
//! scan record, so nothing downstream could count, group, localize or alert on one. What travels
//! now is a code plus the parameters that fill it; the sentence lives in
//! [`TypeMetadataProvider::description`] and is composed in the UI, where `Intl.ListFormat` can do
//! the joining in the reader's own language.
//!
//! Two rules shape the enum below.
//!
//! - **One code per claim.** Each variant is one arm of what used to be a renderer's `match` — one
//!   statement about one kind of failure. Collapsing "credential rejected", "TLS failed" and
//!   "unreachable" into one code would make the metric useless, because which of the three it was
//!   is the entire question an operator asks.
//! - **One warning per occurrence.** A warning names the single thing it is about and carries that
//!   thing's own details. Nothing is summed or flattened before the wire; the UI groups warnings
//!   sharing a code and joins their addresses for display. Aggregating first is what lost
//!   `collected` counts, per-address diagnostics, and the pairing between a credential and the
//!   address it failed at.

pub mod metadata;
pub mod values;

use std::net::IpAddr;

use semver::Version;
use serde::{Deserialize, Deserializer, Serialize};
use strum::{EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;

pub use metadata::WarningRemedy;
pub use values::{ClaimSource, MalformedNeighbourConsequence, SnmpWalkGroup};

/// A single non-fatal finding from one discovery run, about one device, neighbour, or the scan
/// itself.
///
/// Serialized with the code as the tag, so the generated TypeScript is a discriminated union the
/// UI can switch on exhaustively. The derived `Deserialize` reads that shape; the leniency that
/// keeps historical records and pre-coded daemons working lives in [`deserialize_warnings`],
/// which is applied at the one field that holds these.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(tag = "code")]
pub enum DiscoveryWarning {
    // ---- Interface walks -------------------------------------------------
    /// The interface *set* was cut short, so interfaces are genuinely missing.
    #[schema(title = "InterfaceSetCutShort")]
    InterfaceSetCutShort {
        /// The device whose walk fell short.
        #[schema(value_type = String)]
        address: IpAddr,
        /// Interfaces read before the walk stopped.
        collected: u32,
    },
    /// The set was complete and only the attribute columns fell short, so nothing is missing —
    /// some descriptions or speeds are just blank. Kept apart from the above because reporting
    /// this as possible data loss sends people hunting for interfaces that were never absent.
    #[schema(title = "InterfaceDetailsCutShort")]
    InterfaceDetailsCutShort {
        /// The device whose walk fell short.
        #[schema(value_type = String)]
        address: IpAddr,
        /// Interfaces whose attribute columns were read in full.
        collected: u32,
    },

    // ---- SNMP walk shortfalls --------------------------------------------
    /// Stopped at our own entry cap. The device is fine and larger than we read.
    #[schema(title = "SnmpWalkEntryCap")]
    SnmpWalkEntryCap {
        /// The device this group was read from.
        #[schema(value_type = String)]
        address: IpAddr,
        group: SnmpWalkGroup,
        /// Entries per table that collection stops at.
        limit: u32,
    },
    /// The device does not implement this MIB. Not a fault, and no later scan will change it.
    #[schema(title = "SnmpWalkUnsupported")]
    SnmpWalkUnsupported {
        /// The device this group was read from.
        #[schema(value_type = String)]
        address: IpAddr,
        group: SnmpWalkGroup,
    },
    /// The agent answered out of step with what was asked — stale or non-advancing responses.
    #[schema(title = "SnmpWalkDesynchronised")]
    SnmpWalkDesynchronised {
        /// The device this group was read from.
        #[schema(value_type = String)]
        address: IpAddr,
        group: SnmpWalkGroup,
    },
    /// A partial read whose rows are thrown away rather than recorded, so the device contributes
    /// nothing for this group however much it answered.
    #[schema(title = "SnmpWalkPartialDiscarded")]
    SnmpWalkPartialDiscarded {
        /// The device this group was read from.
        #[schema(value_type = String)]
        address: IpAddr,
        group: SnmpWalkGroup,
    },
    /// A partial read whose rows were recorded as far as they got.
    #[schema(title = "SnmpWalkPartialRecorded")]
    SnmpWalkPartialRecorded {
        /// The device this group was read from.
        #[schema(value_type = String)]
        address: IpAddr,
        group: SnmpWalkGroup,
    },
    /// Nothing came back for the root of the bridge MIB, which switches commonly do not implement.
    #[schema(title = "SnmpWalkBridgeMibAbsent")]
    SnmpWalkBridgeMibAbsent {
        /// The device this group was read from.
        #[schema(value_type = String)]
        address: IpAddr,
        group: SnmpWalkGroup,
    },
    /// Nothing came back at all, and the device stopped answering rather than reporting empty.
    #[schema(title = "SnmpWalkNoAnswer")]
    SnmpWalkNoAnswer {
        /// The device this group was read from.
        #[schema(value_type = String)]
        address: IpAddr,
        group: SnmpWalkGroup,
    },

    // ---- Contradicted claims ---------------------------------------------
    /// The device published a count, and the read ended before reaching it.
    #[schema(title = "ClaimedCountReadCutShort")]
    ClaimedCountReadCutShort {
        /// The device that published the count.
        #[schema(value_type = String)]
        address: IpAddr,
        group: SnmpWalkGroup,
        source: ClaimSource,
        /// Rows the device said it had.
        expected: u32,
        /// Rows the read returned.
        observed: u32,
    },
    /// The device published a count, the read finished, and it came up short anyway.
    #[schema(title = "ClaimedCountUnderRead")]
    ClaimedCountUnderRead {
        /// The device that published the count.
        #[schema(value_type = String)]
        address: IpAddr,
        group: SnmpWalkGroup,
        source: ClaimSource,
        /// Rows the device said it had.
        expected: u32,
        /// Rows the read returned.
        observed: u32,
    },
    /// The device declared the capability, and the read ended without returning any.
    #[schema(title = "ClaimedCapabilityReadCutShort")]
    ClaimedCapabilityReadCutShort {
        /// The device that declared the capability.
        #[schema(value_type = String)]
        address: IpAddr,
        group: SnmpWalkGroup,
        source: ClaimSource,
    },
    /// The device declared the capability, the read finished, and it returned none.
    #[schema(title = "ClaimedCapabilityEmpty")]
    ClaimedCapabilityEmpty {
        /// The device that declared the capability.
        #[schema(value_type = String)]
        address: IpAddr,
        group: SnmpWalkGroup,
        source: ClaimSource,
    },

    // ---- LLDP local port numbering ---------------------------------------
    /// Neighbours whose local port matched no interface, so they were discarded entirely.
    #[schema(title = "LldpLocalPortDropped")]
    LldpLocalPortDropped {
        /// The device that reported the neighbours.
        #[schema(value_type = String)]
        address: IpAddr,
        /// Neighbours discarded for want of a matching interface.
        dropped: u32,
        /// Neighbours the device reported in all.
        total: u32,
    },
    /// The same loss, on a device whose own account of its port numbering was only half read.
    ///
    /// Separate from [`Self::LldpLocalPortDropped`] because that one names a cause — the switch
    /// numbering its LLDP ports apart from its interfaces — which is a property of the device and
    /// is not what happened here. Offering both readings in one sentence, as this used to, asks
    /// the operator to pick; one says nothing will ever change and the other says a complete scan
    /// may well place these neighbours (GH #668).
    #[schema(title = "LldpLocalPortDroppedReadCutShort")]
    LldpLocalPortDroppedReadCutShort {
        /// The device that reported the neighbours.
        #[schema(value_type = String)]
        address: IpAddr,
        /// Neighbours discarded for want of a matching interface.
        dropped: u32,
        /// Neighbours the device reported in all.
        total: u32,
    },
    /// Neighbours whose local port could not be identified but did match an interface number, so
    /// they are drawn against a port that may be the wrong one.
    #[schema(title = "LldpLocalPortMisplaced")]
    LldpLocalPortMisplaced {
        /// The device that reported the neighbours.
        #[schema(value_type = String)]
        address: IpAddr,
        /// Neighbours drawn against a port that may be the wrong one.
        misplaced: u32,
    },

    // ---- Malformed neighbour records -------------------------------------
    /// The column carrying the identifier stopped early, so a rescan may recover these.
    MalformedNeighboursWalkCutShort(MalformedNeighbours),
    /// Rows that never appeared in the identifying column at all.
    MalformedNeighboursGhostRows(MalformedNeighbours),
    /// Neighbours listed and then never given an identifier.
    MalformedNeighboursIncompleteRecords(MalformedNeighbours),
    /// The identifying column held a value of a type it cannot hold.
    MalformedNeighboursUnexpectedType(MalformedNeighbours),
    /// The record's position in the neighbour table could not be read.
    MalformedNeighboursUnreadableIndex(MalformedNeighbours),

    // ---- Whole-device outcomes -------------------------------------------
    /// SNMP answered and every table came back empty.
    #[schema(title = "SnmpCollectedNothing")]
    SnmpCollectedNothing {
        /// The device that answered.
        #[schema(value_type = String)]
        address: IpAddr,
    },
    /// The device answered correctly and persisting its VLANs failed.
    #[schema(title = "VlanRecordingFailed")]
    VlanRecordingFailed {
        /// The device whose VLANs could not be recorded.
        #[schema(value_type = String)]
        address: IpAddr,
    },
    /// Two integrations that each read the device's full interface table both collected it, and
    /// their interface sets were merged.
    ///
    /// A supported configuration, not a fault. It is reported because the merge keeps one row
    /// per port: where both describe a port, the first to answer supplies the row and its
    /// neighbours, and the second's are dropped. Someone chasing a missing link needs to know
    /// that happened.
    #[schema(title = "EqualReachIntegrationsMerged")]
    EqualReachIntegrationsMerged {
        /// The device both integrations read.
        #[schema(value_type = String)]
        address: IpAddr,
        /// The integration that answered first. Its row stands on every port both describe.
        first: CredentialQueryPayloadDiscriminants,
        /// The integration that answered second.
        second: CredentialQueryPayloadDiscriminants,
    },

    // ---- Credential issues -----------------------------------------------
    /// The credential's address is not on any subnet this scan covers.
    #[schema(title = "CredentialTargetNotScanned")]
    CredentialTargetNotScanned {
        /// The address the credential is bound to.
        #[schema(value_type = String)]
        address: IpAddr,
        integration: CredentialQueryPayloadDiscriminants,
        /// The stored credential bound to that address. See [`CredentialAttempt::credential_id`].
        #[serde(default)]
        #[schema(required)]
        credential_id: Option<Uuid>,
    },
    /// Nothing answered at the credential's address during the scan.
    #[schema(title = "CredentialTargetNotResponding")]
    CredentialTargetNotResponding {
        /// The address the credential is bound to.
        #[schema(value_type = String)]
        address: IpAddr,
        integration: CredentialQueryPayloadDiscriminants,
        /// The stored credential bound to that address. See [`CredentialAttempt::credential_id`].
        #[serde(default)]
        #[schema(required)]
        credential_id: Option<Uuid>,
    },
    /// The port the credential needs was not open, so it was never tried.
    #[schema(title = "CredentialGateClosed")]
    CredentialGateClosed {
        /// The address the credential is bound to.
        #[schema(value_type = String)]
        address: IpAddr,
        integration: CredentialQueryPayloadDiscriminants,
        /// The ports that had to be open for the probe to run.
        ports: Vec<u16>,
        /// The stored credential that needed those ports. See [`CredentialAttempt::credential_id`].
        #[serde(default)]
        #[schema(required)]
        credential_id: Option<Uuid>,
    },
    /// The credential was refused.
    CredentialRejected(CredentialAttempt),
    /// The credential is incomplete and could not be used.
    CredentialMalformed(CredentialAttempt),
    /// TLS could not be negotiated.
    CredentialTlsFailed(CredentialAttempt),
    /// Something answered that is not the expected service.
    CredentialNotThisService(CredentialAttempt),
    /// Authenticated, then failed while collecting.
    CredentialCollectionFailed(CredentialAttempt),
    /// Authenticated, then ran out of time while collecting.
    CredentialCollectionTimedOut(CredentialAttempt),
    /// Nothing was reachable at the address.
    CredentialUnreachable(CredentialAttempt),
    /// The attempt timed out before anything answered.
    CredentialTimedOut(CredentialAttempt),

    // ---- Scan-level ------------------------------------------------------
    /// Addresses on a routed subnet completed a TCP handshake and then answered nothing, so no
    /// host was recorded at any of them.
    ///
    /// The shape a middlebox in the path produces: a firewall session helper, a load balancer or a
    /// scrubbing appliance answering on behalf of addresses with nothing behind them. One warning
    /// per subnet rather than per address — the case that raises it raises it for every address in
    /// the range, and 254 copies of a sentence is not 254 findings.
    #[schema(title = "ConnectionsWithoutProtocolResponse")]
    ConnectionsWithoutProtocolResponse {
        /// The subnet the addresses are in.
        cidr: String,
        /// How many addresses in it answered a connect and nothing else.
        declined: u32,
        /// The ports that completed a handshake, most frequent first.
        ports: Vec<u16>,
    },
    /// The run hit its global time limit, with an estimate of the work left.
    #[schema(title = "ScanTimeLimitWithEstimate")]
    ScanTimeLimitWithEstimate {
        /// The limit the run hit, in hours.
        hours: u32,
        /// Hosts still queued when the run stopped.
        hosts_not_scanned: u32,
        /// Estimated minutes of work left at that point.
        minutes_remaining: u32,
    },
    /// The run hit its global time limit, with no usable estimate.
    #[schema(title = "ScanTimeLimit")]
    ScanTimeLimit {
        /// The limit the run hit, in hours.
        hours: u32,
        /// Hosts still queued when the run stopped.
        hosts_not_scanned: u32,
    },
    /// The PROFINET DCP sweep did not report back within its budget, so the run finished
    /// without the devices only it would have found.
    #[schema(title = "DcpSweepTimedOut")]
    DcpSweepTimedOut {
        /// The budget the sweep had, in seconds.
        seconds: u32,
    },
    /// The ICMP sweep did not report back within its budget, so hosts that answer only to ping
    /// were not scanned.
    #[schema(title = "IcmpSweepTimedOut")]
    IcmpSweepTimedOut {
        /// The budget the sweep had, in seconds.
        seconds: u32,
    },
    /// Reverse DNS lookups that got no answer in time, so those hosts have no DNS hostname.
    #[schema(title = "ReverseDnsTimedOut")]
    ReverseDnsTimedOut {
        /// How many lookups went unanswered.
        count: u32,
    },

    // ---- Server-side LLDP/CDP resolution ---------------------------------
    /// The advertised identifier matches no host on this network.
    LldpNeighbourNotFound(UnmatchedNeighbour),
    /// The advertised identifier matches several hosts, so none can be picked.
    LldpNeighbourAmbiguous(UnmatchedNeighbour),
    /// The far end resolved, and its port id is of a subtype there is no lookup for.
    LldpPortNoStrategy(UnresolvedPort),
    /// The far end resolved, and none of its ports matches the advertised port id.
    LldpPortNotFound(UnresolvedPort),
    /// The far end resolved, and several of its ports match, so it identifies none.
    LldpPortAmbiguous(UnresolvedPort),
    /// A range was created from the addresses unplaceable far ends publish for themselves.
    ProvisionalSubnetInferred(ProvisionalSubnet),
    /// Link resolution ran out of its time budget, so this scan's links are incomplete.
    ///
    /// The pass runs on the completion request, which the daemon will abandon at its own timeout —
    /// so it is bounded, and stopping has to be *said* rather than logged. Without this the
    /// degraded result is indistinguishable from a network that genuinely has no neighbours.
    #[schema(title = "NeighbourResolutionIncomplete")]
    NeighbourResolutionIncomplete {
        /// Seconds the pass was allowed before it was stopped.
        budget_seconds: u32,
        /// Interfaces carrying neighbour data it was working through, so the reader can tell
        /// "this network is large" from "something is wrong".
        neighbours: u32,
    },
    /// FDB link resolution ran out of its time budget, so this scan's FDB-derived links are
    /// incomplete.
    ///
    /// Distinct from `NeighbourResolutionIncomplete`: that pass runs first and this one is
    /// strictly sequenced after it, so a reader needs to know which pass stopped to judge how
    /// much of Physical Topology is affected.
    #[schema(title = "FdbResolutionIncomplete")]
    FdbResolutionIncomplete {
        /// Seconds the pass was allowed before it was stopped.
        budget_seconds: u32,
        /// Interfaces with an unresolved single-MAC FDB entry it was working through.
        interfaces: u32,
    },

    // ---- Daemon ----------------------------------------------------------
    /// The daemon submitted this scan in a wire format a current daemon no longer produces.
    ///
    /// Server-side finding, raised where the raw request body is read. The server translates the
    /// old format, so nothing was lost — this says the translation happened, which is the only
    /// evidence an operator ever gets that a daemon is behind. Which superseded format it was is
    /// deliberately not carried, and neither is the fact that the translation worked: the reader
    /// needs one thing from this, which is that the daemon is out of date.
    ///
    /// The daemon to upgrade is the session's own — the UI reads `daemon_id` off the payload this
    /// rides on rather than repeating it here, which is what lets the row deep-link to that
    /// daemon's upgrade modal.
    ///
    /// Ends with the compatibility window. When the enforced floor rises past the last release
    /// that sends an old format, the translations go and this goes with them.
    #[schema(title = "OutdatedDaemonFormat")]
    OutdatedDaemonFormat {
        /// The submitting daemon's version, so the reader knows which one to upgrade. `None` for
        /// a daemon too old to report one at all.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schema(
            value_type = Option<String>,
            pattern = r"^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$",
            example = "0.17.14"
        )]
        daemon_version: Option<Version>,
    },

    // ---- Meta ------------------------------------------------------------
    /// The run produced more warnings than the scan record holds. Emitted rather than dropping
    /// the tail silently — a list that simply stops reads as though that was all of them.
    #[schema(title = "WarningsTruncated")]
    WarningsTruncated {
        /// Warnings dropped past the record's cap.
        elided: u32,
    },
    /// A warning this binary does not recognise: a bare string from a historical record or a
    /// pre-coded daemon, or a code from a newer one. Carries the original text so scan history
    /// keeps rendering; the code itself is what reaches the metric, never `detail`.
    #[schema(title = "Unknown")]
    Unknown {
        /// The original warning text, rendered as-is.
        detail: String,
    },
}

/// Neighbour records discarded for want of the identifier that matches the far end.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct MalformedNeighbours {
    /// The device that reported the records.
    #[schema(value_type = String)]
    pub address: IpAddr,
    pub group: SnmpWalkGroup,
    /// Records thrown away for want of a usable identifier.
    pub discarded: u32,
    /// Records that survived, which is what decides whether this cost the device some of its
    /// topology or all of it.
    pub kept: u32,
    pub consequence: MalformedNeighbourConsequence,
}

/// One credential's attempt against one address, and what the client library said about it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct CredentialAttempt {
    /// The address the credential was tried against.
    #[schema(value_type = String)]
    pub address: IpAddr,
    pub integration: CredentialQueryPayloadDiscriminants,
    /// The stored credential this attempt used, so the report can open it rather than sending the
    /// reader to the list to find it. `integration` narrows to a type; on a network with two SNMP
    /// communities the address does not narrow to a record, which is what this is for.
    ///
    /// `None` on a warning written before the id was carried, on one from a daemon that predates
    /// it, and on the daemon's own injected "public" fallback, which has no stored row.
    #[serde(default)]
    #[schema(required)]
    pub credential_id: Option<Uuid>,
    /// The library's own diagnostic — free text, so it can only ever be displayed. It is the one
    /// thing the code cannot supersede: the code says which failure mode, this says what actually
    /// came back ("connection refused (os error 111)"), and it is now attributable to this one
    /// address rather than being the first message of a whole batch.
    #[schema(required)]
    pub detail: Option<String>,
}

/// A neighbour advertised by a local interface whose far end could not be placed on a host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct UnmatchedNeighbour {
    /// The local device that saw the neighbour, not the far end — the far end is what could not
    /// be identified.
    pub host_id: Uuid,
    /// The local interface that advertised the neighbour.
    pub if_descr: String,
    /// The chassis ID (LLDP) or device id (CDP) that did not identify one host.
    pub identifier: String,
    /// The far end's advertised `sysName`, where it sent one.
    #[schema(required)]
    pub sys_name: Option<String>,
    /// The address the far end published for itself — `lldpRemManAddr`, a `NetworkAddress` port
    /// id, or `cdpCacheAddress` — where it sent a usable one.
    ///
    /// Carried because it is the difference between two reports that otherwise read identically:
    /// "the far end told us where it lives and this network holds no such address" is a device
    /// nobody has scanned, while "it told us nothing" is a device that cannot be placed no matter
    /// how much of the network is scanned. Without it, deciding which of the two a fleet is
    /// looking at costs another round trip to the operator (GH #668).
    #[schema(required)]
    pub address: Option<String>,
}

/// A range inferred from where unplaceable far ends say they live.
///
/// Carries the evidence rather than just the verdict, because "we invented a subnet" is not a
/// sentence an operator can act on: which of their devices saw it, on which ports, and what those
/// far ends call themselves is what lets them recognise a segment as real, correct the range, or
/// see that it is a device with a factory-default address.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct ProvisionalSubnet {
    /// The range, in CIDR notation.
    pub cidr: String,
    /// The subnet row created for it, so the UI can link straight to it.
    pub subnet_id: Uuid,
    /// The addresses that motivated it, in the order observed.
    pub addresses: Vec<String>,
    /// The far ends' own `sysName`s, where they sent any — the labels an operator recognises.
    pub sys_names: Vec<String>,
    /// The local devices that saw them.
    pub seen_by_host_ids: Vec<Uuid>,
    /// Whether a shared VLAN widened the range past the conventional prefix, rather than it being
    /// the `/24` (or `/64`) a lone address is assumed to sit in. The difference is how much the
    /// range rests on evidence versus on convention.
    pub widened_by_vlan: bool,
}

/// A neighbour whose far-end host resolved but whose far-end *port* did not.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
pub struct UnresolvedPort {
    /// The local device that saw the neighbour, and the port it saw it on.
    pub host_id: Uuid,
    /// The local interface that advertised the neighbour.
    pub if_descr: String,
    /// The far-end device, already resolved — this is what makes it distinct from
    /// [`UnmatchedNeighbour`].
    pub remote_host_id: Uuid,
    /// The advertised port id in `Debug` form, which carries subtype and value together
    /// (`MacAddress("00:ad:24:af:4e:00")`, `InterfaceName("2")`). Both halves are needed: the
    /// subtype says which tier ran and the value says what it looked for.
    #[schema(required)]
    pub port_id: Option<String>,
    /// `lldpRemPortDesc`, the last-resort tier. Present because "the id failed and the description
    /// was empty" and "both were tried and neither matched" call for different fixes.
    #[schema(required)]
    pub port_desc: Option<String>,
}

/// The stable identity of a warning: what crosses the wire as the tag, what labels the metric,
/// and what the UI resolves its sentence from.
///
/// Hand-declared rather than generated by `EnumDiscriminants` because it has to carry its own
/// derives to satisfy `Operation`, and a generated enum cannot.
/// [`DiscoveryWarning::code`] is an exhaustive match, so the two cannot drift.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    ToSchema,
    EnumIter,
    VariantNames,
    IntoStaticStr,
    strum::Display,
    strum::AsRefStr,
    strum::EnumDiscriminants,
)]
#[strum_discriminants(
    name(DiscoveryWarningCodeKind),
    derive(
        Hash,
        EnumIter,
        strum::Display,
        strum::AsRefStr,
        Serialize,
        Deserialize
    )
)]
pub enum DiscoveryWarningCode {
    InterfaceSetCutShort,
    InterfaceDetailsCutShort,
    SnmpWalkEntryCap,
    SnmpWalkUnsupported,
    SnmpWalkDesynchronised,
    SnmpWalkPartialDiscarded,
    SnmpWalkPartialRecorded,
    SnmpWalkBridgeMibAbsent,
    SnmpWalkNoAnswer,
    ClaimedCountReadCutShort,
    ClaimedCountUnderRead,
    ClaimedCapabilityReadCutShort,
    ClaimedCapabilityEmpty,
    LldpLocalPortDropped,
    LldpLocalPortDroppedReadCutShort,
    LldpLocalPortMisplaced,
    MalformedNeighboursWalkCutShort,
    MalformedNeighboursGhostRows,
    MalformedNeighboursIncompleteRecords,
    MalformedNeighboursUnexpectedType,
    MalformedNeighboursUnreadableIndex,
    SnmpCollectedNothing,
    VlanRecordingFailed,
    EqualReachIntegrationsMerged,
    CredentialTargetNotScanned,
    CredentialTargetNotResponding,
    CredentialGateClosed,
    CredentialRejected,
    CredentialMalformed,
    CredentialTlsFailed,
    CredentialNotThisService,
    CredentialCollectionFailed,
    CredentialCollectionTimedOut,
    CredentialUnreachable,
    CredentialTimedOut,
    ConnectionsWithoutProtocolResponse,
    ScanTimeLimitWithEstimate,
    ScanTimeLimit,
    DcpSweepTimedOut,
    IcmpSweepTimedOut,
    ReverseDnsTimedOut,
    LldpNeighbourNotFound,
    LldpNeighbourAmbiguous,
    LldpPortNoStrategy,
    LldpPortNotFound,
    LldpPortAmbiguous,
    ProvisionalSubnetInferred,
    NeighbourResolutionIncomplete,
    FdbResolutionIncomplete,
    OutdatedDaemonFormat,
    WarningsTruncated,
    /// Absorbs a code from a newer binary. Fieldless, so `#[serde(other)]` applies — the text of
    /// an unrecognised warning rides on [`DiscoveryWarning::Unknown`] instead, where no metric
    /// label or analytics property can reach it.
    #[serde(other)]
    Unknown,
}

impl DiscoveryWarning {
    /// This warning's stable code. Exhaustive, so a new variant cannot ship without one.
    pub fn code(&self) -> DiscoveryWarningCode {
        match self {
            Self::InterfaceSetCutShort { .. } => DiscoveryWarningCode::InterfaceSetCutShort,
            Self::InterfaceDetailsCutShort { .. } => DiscoveryWarningCode::InterfaceDetailsCutShort,
            Self::SnmpWalkEntryCap { .. } => DiscoveryWarningCode::SnmpWalkEntryCap,
            Self::SnmpWalkUnsupported { .. } => DiscoveryWarningCode::SnmpWalkUnsupported,
            Self::SnmpWalkDesynchronised { .. } => DiscoveryWarningCode::SnmpWalkDesynchronised,
            Self::SnmpWalkPartialDiscarded { .. } => DiscoveryWarningCode::SnmpWalkPartialDiscarded,
            Self::SnmpWalkPartialRecorded { .. } => DiscoveryWarningCode::SnmpWalkPartialRecorded,
            Self::SnmpWalkBridgeMibAbsent { .. } => DiscoveryWarningCode::SnmpWalkBridgeMibAbsent,
            Self::SnmpWalkNoAnswer { .. } => DiscoveryWarningCode::SnmpWalkNoAnswer,
            Self::ClaimedCountReadCutShort { .. } => DiscoveryWarningCode::ClaimedCountReadCutShort,
            Self::ClaimedCountUnderRead { .. } => DiscoveryWarningCode::ClaimedCountUnderRead,
            Self::ClaimedCapabilityReadCutShort { .. } => {
                DiscoveryWarningCode::ClaimedCapabilityReadCutShort
            }
            Self::ClaimedCapabilityEmpty { .. } => DiscoveryWarningCode::ClaimedCapabilityEmpty,
            Self::LldpLocalPortDropped { .. } => DiscoveryWarningCode::LldpLocalPortDropped,
            Self::LldpLocalPortDroppedReadCutShort { .. } => {
                DiscoveryWarningCode::LldpLocalPortDroppedReadCutShort
            }
            Self::LldpLocalPortMisplaced { .. } => DiscoveryWarningCode::LldpLocalPortMisplaced,
            Self::MalformedNeighboursWalkCutShort(_) => {
                DiscoveryWarningCode::MalformedNeighboursWalkCutShort
            }
            Self::MalformedNeighboursGhostRows(_) => {
                DiscoveryWarningCode::MalformedNeighboursGhostRows
            }
            Self::MalformedNeighboursIncompleteRecords(_) => {
                DiscoveryWarningCode::MalformedNeighboursIncompleteRecords
            }
            Self::MalformedNeighboursUnexpectedType(_) => {
                DiscoveryWarningCode::MalformedNeighboursUnexpectedType
            }
            Self::MalformedNeighboursUnreadableIndex(_) => {
                DiscoveryWarningCode::MalformedNeighboursUnreadableIndex
            }
            Self::SnmpCollectedNothing { .. } => DiscoveryWarningCode::SnmpCollectedNothing,
            Self::VlanRecordingFailed { .. } => DiscoveryWarningCode::VlanRecordingFailed,
            Self::EqualReachIntegrationsMerged { .. } => {
                DiscoveryWarningCode::EqualReachIntegrationsMerged
            }
            Self::CredentialTargetNotScanned { .. } => {
                DiscoveryWarningCode::CredentialTargetNotScanned
            }
            Self::CredentialTargetNotResponding { .. } => {
                DiscoveryWarningCode::CredentialTargetNotResponding
            }
            Self::CredentialGateClosed { .. } => DiscoveryWarningCode::CredentialGateClosed,
            Self::CredentialRejected(_) => DiscoveryWarningCode::CredentialRejected,
            Self::CredentialMalformed(_) => DiscoveryWarningCode::CredentialMalformed,
            Self::CredentialTlsFailed(_) => DiscoveryWarningCode::CredentialTlsFailed,
            Self::CredentialNotThisService(_) => DiscoveryWarningCode::CredentialNotThisService,
            Self::CredentialCollectionFailed(_) => DiscoveryWarningCode::CredentialCollectionFailed,
            Self::CredentialCollectionTimedOut(_) => {
                DiscoveryWarningCode::CredentialCollectionTimedOut
            }
            Self::CredentialUnreachable(_) => DiscoveryWarningCode::CredentialUnreachable,
            Self::CredentialTimedOut(_) => DiscoveryWarningCode::CredentialTimedOut,
            Self::ConnectionsWithoutProtocolResponse { .. } => {
                DiscoveryWarningCode::ConnectionsWithoutProtocolResponse
            }
            Self::ScanTimeLimitWithEstimate { .. } => {
                DiscoveryWarningCode::ScanTimeLimitWithEstimate
            }
            Self::ScanTimeLimit { .. } => DiscoveryWarningCode::ScanTimeLimit,
            Self::DcpSweepTimedOut { .. } => DiscoveryWarningCode::DcpSweepTimedOut,
            Self::IcmpSweepTimedOut { .. } => DiscoveryWarningCode::IcmpSweepTimedOut,
            Self::ReverseDnsTimedOut { .. } => DiscoveryWarningCode::ReverseDnsTimedOut,
            Self::LldpNeighbourNotFound(_) => DiscoveryWarningCode::LldpNeighbourNotFound,
            Self::LldpNeighbourAmbiguous(_) => DiscoveryWarningCode::LldpNeighbourAmbiguous,
            Self::LldpPortNoStrategy(_) => DiscoveryWarningCode::LldpPortNoStrategy,
            Self::LldpPortNotFound(_) => DiscoveryWarningCode::LldpPortNotFound,
            Self::LldpPortAmbiguous(_) => DiscoveryWarningCode::LldpPortAmbiguous,
            Self::ProvisionalSubnetInferred(_) => DiscoveryWarningCode::ProvisionalSubnetInferred,
            Self::NeighbourResolutionIncomplete { .. } => {
                DiscoveryWarningCode::NeighbourResolutionIncomplete
            }
            Self::FdbResolutionIncomplete { .. } => DiscoveryWarningCode::FdbResolutionIncomplete,
            Self::OutdatedDaemonFormat { .. } => DiscoveryWarningCode::OutdatedDaemonFormat,
            Self::WarningsTruncated { .. } => DiscoveryWarningCode::WarningsTruncated,
            Self::Unknown { .. } => DiscoveryWarningCode::Unknown,
        }
    }

    /// The integration this warning came from, for the metric's second label.
    ///
    /// `None` is the honest answer for the scan-level and server-side findings — they belong to
    /// the pipeline and to link resolution, not to any one integration — and becomes the `none`
    /// label rather than being dropped.
    pub fn integration(&self) -> Option<CredentialQueryPayloadDiscriminants> {
        match self {
            // Every SNMP-family record is produced by the SNMP integration and nothing else.
            Self::InterfaceSetCutShort { .. }
            | Self::InterfaceDetailsCutShort { .. }
            | Self::SnmpWalkEntryCap { .. }
            | Self::SnmpWalkUnsupported { .. }
            | Self::SnmpWalkDesynchronised { .. }
            | Self::SnmpWalkPartialDiscarded { .. }
            | Self::SnmpWalkPartialRecorded { .. }
            | Self::SnmpWalkBridgeMibAbsent { .. }
            | Self::SnmpWalkNoAnswer { .. }
            | Self::ClaimedCountReadCutShort { .. }
            | Self::ClaimedCountUnderRead { .. }
            | Self::ClaimedCapabilityReadCutShort { .. }
            | Self::ClaimedCapabilityEmpty { .. }
            | Self::LldpLocalPortDropped { .. }
            | Self::LldpLocalPortDroppedReadCutShort { .. }
            | Self::LldpLocalPortMisplaced { .. }
            | Self::MalformedNeighboursWalkCutShort(_)
            | Self::MalformedNeighboursGhostRows(_)
            | Self::MalformedNeighboursIncompleteRecords(_)
            | Self::MalformedNeighboursUnexpectedType(_)
            | Self::MalformedNeighboursUnreadableIndex(_)
            | Self::SnmpCollectedNothing { .. }
            | Self::VlanRecordingFailed { .. } => Some(CredentialQueryPayloadDiscriminants::Snmp),

            Self::CredentialTargetNotScanned { integration, .. }
            | Self::CredentialTargetNotResponding { integration, .. }
            | Self::CredentialGateClosed { integration, .. } => Some(*integration),

            Self::CredentialRejected(a)
            | Self::CredentialMalformed(a)
            | Self::CredentialTlsFailed(a)
            | Self::CredentialNotThisService(a)
            | Self::CredentialCollectionFailed(a)
            | Self::CredentialCollectionTimedOut(a)
            | Self::CredentialUnreachable(a)
            | Self::CredentialTimedOut(a) => Some(a.integration),

            // Two integrations, and neither one produced it: the merge in the daemon pipeline did.
            // Naming either would count it against an integration that did nothing wrong.
            Self::EqualReachIntegrationsMerged { .. }
            | Self::ConnectionsWithoutProtocolResponse { .. }
            | Self::ScanTimeLimitWithEstimate { .. }
            | Self::ScanTimeLimit { .. }
            | Self::DcpSweepTimedOut { .. }
            | Self::IcmpSweepTimedOut { .. }
            | Self::ReverseDnsTimedOut { .. }
            | Self::LldpNeighbourNotFound(_)
            | Self::LldpNeighbourAmbiguous(_)
            | Self::LldpPortNoStrategy(_)
            | Self::LldpPortNotFound(_)
            | Self::LldpPortAmbiguous(_)
            | Self::ProvisionalSubnetInferred(_)
            | Self::NeighbourResolutionIncomplete { .. }
            | Self::FdbResolutionIncomplete { .. }
            | Self::OutdatedDaemonFormat { .. }
            | Self::WarningsTruncated { .. }
            | Self::Unknown { .. } => None,
        }
    }
}

/// Read the `warnings` array, tolerating everything four deploy directions can put in it.
///
/// Three shapes arrive here and all three have to keep the scan record readable:
///
/// - a **bare JSON string**, from a historical row written before warnings were coded or from a
///   daemon that predates them — becomes `Unknown` carrying that sentence, so scan history goes on
///   rendering exactly as it did;
/// - a **known code**, which deserializes normally;
/// - an **unrecognised code** from a newer daemon, which becomes `Unknown` carrying the raw JSON.
///
/// Per-element rather than per-array, so one unreadable warning costs that warning and not the
/// whole terminal payload. This is the reason no data migration is needed: a backfill could only
/// rewrite the old sentences into `Unknown` objects — it cannot recover codes from prose — and it
/// would not remove the need for this path anyway, since old daemons keep posting strings.
pub fn deserialize_warnings<'de, D>(deserializer: D) -> Result<Vec<DiscoveryWarning>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
    Ok(raw.into_iter().map(warning_from_value).collect())
}

fn warning_from_value(value: serde_json::Value) -> DiscoveryWarning {
    if let serde_json::Value::String(detail) = value {
        return DiscoveryWarning::Unknown { detail };
    }
    serde_json::from_value::<DiscoveryWarning>(value.clone()).unwrap_or(DiscoveryWarning::Unknown {
        detail: value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::daemons::r#impl::api::DiscoveryUpdatePayload;
    use strum::IntoEnumIterator;

    fn warnings_of(json: serde_json::Value) -> Vec<DiscoveryWarning> {
        #[derive(Deserialize)]
        struct Holder {
            #[serde(default, deserialize_with = "deserialize_warnings")]
            warnings: Vec<DiscoveryWarning>,
        }
        serde_json::from_value::<Holder>(serde_json::json!({ "warnings": json }))
            .expect("the warnings field must never fail to deserialize")
            .warnings
    }

    /// The three shapes the four deploy directions put in this field, and the reason there is no
    /// data migration: every stored session written before warnings were coded holds bare English
    /// sentences, and `from_row` hard-errors rather than skipping a row it cannot read. A backfill
    /// could only rewrite those sentences into `Unknown` objects — it cannot recover a code from
    /// prose — and old daemons would keep posting strings regardless.
    #[test]
    fn strings_and_unrecognised_codes_both_survive_as_unknown() {
        let mixed = warnings_of(serde_json::json!([
            "Scan hit its time limit (4h) — 12 host(s) not scanned.",
            { "code": "SnmpCollectedNothing", "address": "10.0.0.1" },
            { "code": "SomethingANewerDaemonKnowsAbout", "address": "10.0.0.2" },
        ]));

        assert_eq!(
            mixed.iter().map(DiscoveryWarning::code).collect::<Vec<_>>(),
            vec![
                DiscoveryWarningCode::Unknown,
                DiscoveryWarningCode::SnmpCollectedNothing,
                DiscoveryWarningCode::Unknown,
            ]
        );

        let DiscoveryWarning::Unknown { detail } = &mixed[0] else {
            panic!("a legacy string must keep its text: {:?}", mixed[0]);
        };
        assert_eq!(
            detail,
            "Scan hit its time limit (4h) — 12 host(s) not scanned."
        );
    }

    /// One unreadable warning costs that warning, never the terminal payload. A scan whose
    /// completion is lost because of a malformed entry in a *non-fatal* list is the failure this
    /// per-element handling exists to prevent.
    #[test]
    fn one_unreadable_warning_does_not_take_the_payload_with_it() {
        let warnings = warnings_of(serde_json::json!([
            { "code": "SnmpCollectedNothing", "address": "not-an-address" },
            { "code": "SnmpCollectedNothing", "address": "10.0.0.1" },
        ]));

        assert_eq!(
            warnings
                .iter()
                .map(DiscoveryWarning::code)
                .collect::<Vec<_>>(),
            vec![
                DiscoveryWarningCode::Unknown,
                DiscoveryWarningCode::SnmpCollectedNothing
            ]
        );
    }

    /// The payload an old daemon posts, in full. It reaches the same handler as a current one, and
    /// a 422 here loses the whole terminal update — the run never records as complete, and its
    /// scanned entities go with it.
    #[test]
    fn a_pre_coded_daemons_payload_still_deserializes() {
        let payload: DiscoveryUpdatePayload = serde_json::from_value(serde_json::json!({
            "session_id": "00000000-0000-0000-0000-000000000001",
            "daemon_id": "00000000-0000-0000-0000-000000000002",
            "network_id": "00000000-0000-0000-0000-000000000003",
            "phase": "Complete",
            "discovery_type": { "type": "Network", "subnet_ids": null },
            "progress": 100,
            "error": null,
            "warnings": ["The SNMP queries credential for 10.0.0.5 was refused."],
            "started_at": null,
            "finished_at": null,
        }))
        .expect("an old daemon's terminal payload must not 422");

        assert_eq!(payload.warnings.len(), 1);
        assert_eq!(payload.warnings[0].code(), DiscoveryWarningCode::Unknown);
    }

    /// A coded credential warning recorded before `credential_id` existed keeps its code.
    ///
    /// The field is optional precisely so this stays true. Were it required, the element would
    /// fail to parse and `warning_from_value` would fold it into `Unknown` — which loses the code,
    /// the rung it is filed under, its sentence and its action, and renders the row as raw JSON.
    /// That would hit every historical row *and* every warning from a daemon not yet on this
    /// release, since the warning is built daemon-side.
    #[test]
    fn a_credential_warning_from_before_ids_keeps_its_code_and_carries_none() {
        let warnings = warnings_of(serde_json::json!([
            {
                "code": "CredentialMalformed",
                "address": "192.168.4.187",
                "integration": "Snmp",
                "detail": "Failed to read community from public: No such file or directory",
            },
            {
                "code": "CredentialTargetNotResponding",
                "address": "10.0.0.9",
                "integration": "UnifiController",
            },
        ]));

        assert_eq!(
            warnings.iter().map(|w| w.code()).collect::<Vec<_>>(),
            vec![
                DiscoveryWarningCode::CredentialMalformed,
                DiscoveryWarningCode::CredentialTargetNotResponding,
            ],
            "an old row must keep its code rather than degrading to Unknown: {warnings:?}"
        );

        match &warnings[0] {
            DiscoveryWarning::CredentialMalformed(attempt) => {
                assert_eq!(attempt.credential_id, None);
                // The rest of the payload still reads, so the row renders as it always did.
                assert_eq!(attempt.address.to_string(), "192.168.4.187");
                assert!(attempt.detail.is_some());
            }
            other => panic!("expected CredentialMalformed, got {other:?}"),
        }
        assert!(matches!(
            warnings[1],
            DiscoveryWarning::CredentialTargetNotResponding {
                credential_id: None,
                ..
            }
        ));
    }

    /// The other direction: a warning that does carry an id round-trips it, so the field survives
    /// the trip through `discovery.run_type` rather than being dropped on write or read.
    #[test]
    fn a_credential_id_survives_a_round_trip_through_the_persisted_shape() {
        let credential_id = Uuid::from_u128(0x5ca0);
        let warning = DiscoveryWarning::CredentialMalformed(CredentialAttempt {
            address: "192.168.4.187".parse().unwrap(),
            integration: CredentialQueryPayloadDiscriminants::Snmp,
            detail: None,
            credential_id: Some(credential_id),
        });

        let back = warnings_of(serde_json::json!([serde_json::to_value(&warning).unwrap()]));

        assert_eq!(back, vec![warning]);
        assert!(matches!(
            &back[0],
            DiscoveryWarning::CredentialMalformed(a) if a.credential_id == Some(credential_id)
        ));
    }

    /// Every code round-trips through the tag it serializes as, which is also the metric label and
    /// the fixture id the UI resolves its sentence from. A code whose tag does not deserialize
    /// back to itself would silently become `Unknown` on the next read of its own record.
    #[test]
    fn every_code_round_trips_through_its_serialized_name() {
        for code in DiscoveryWarningCode::iter() {
            let json = serde_json::to_value(code).expect("serializable");
            let back: DiscoveryWarningCode =
                serde_json::from_value(json.clone()).expect("deserializable");
            assert_eq!(back, code, "{json} did not round-trip");
        }
    }

    /// The metric's `integration` label has to be bounded, and `Unknown` is the one code that can
    /// carry arbitrary text. It must never name an integration, or the label picks up whatever a
    /// newer daemon or a historical row happened to contain.
    #[test]
    fn an_unknown_warning_names_no_integration() {
        let warning = DiscoveryWarning::Unknown {
            detail: "anything at all".to_string(),
        };
        assert_eq!(warning.integration(), None);
    }
}
