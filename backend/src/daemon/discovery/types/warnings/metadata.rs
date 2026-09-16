//! The English behind each code, and the contract the UI fills it from.
//!
//! [`TypeMetadataProvider::description`] is a *template*: named slots the UI substitutes from the
//! warnings carrying that code. It reaches the browser as `meta_warning_codes_<Code>_description`
//! through the fixture and paraglide, which is what makes a scan warning translatable for the
//! first time.
//!
//! Slot names are declared once, by [`DiscoveryWarningCode::slots`], and published in the
//! fixture's `metadata` so the UI renderer is checked against the same list the template is. Two
//! things enforce the agreement rather than author discipline: `slots()` is an exhaustive match,
//! so a new code cannot compile without declaring its slots, and the test at the bottom of this
//! file walks every code and fails if a template's slots and its declared slots disagree.
//!
//! The joining is deliberately *not* done here. `{addresses}` arrives already joined by
//! `Intl.ListFormat` in the reader's locale, which is strictly better than the English "A, B, and
//! C" this code used to build.

use serde::{Deserialize, Serialize};
use serde_json::json;
use strum::{EnumIter, IntoStaticStr, VariantNames};
use utoipa::ToSchema;

use super::DiscoveryWarningCode;
use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};

/// How much a warning cost the scan, which is what its colour and icon say at a glance.
///
/// Deliberately *not* the hierarchy the warning list is organised by. It measures what the scan
/// lost, which is a different question from what the reader has to do about it, and the two come
/// apart hard: every credential failure is `Lost`, and so is every malformed-neighbour record
/// whose own sentence ends "Rescanning will not change this". [`WarningRemedy`] is the axis that
/// orders the list; this one stays the glyph on each row, which is what it was written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    /// Data is missing or a credential does not work. Acting on it changes the next scan.
    Lost,
    /// Something is incomplete or uncertain, and a later scan may well fix it.
    Degraded,
    /// A fact about the device, not a fault. Nothing to fix.
    Informational,
}

/// What a warning asks of the person reading it.
///
/// The rung a code sits on *is* its instruction, which is why the warning list is grouped by this
/// and not by [`Severity`]: a reader scanning a completed run needs to know whether they are being
/// asked for something before they need to know how much data was lost. It reaches the UI as each
/// code's `category`, with this enum's own registry supplying the heading — the same shape
/// `service-definitions.json` and `service-categories.json` use.
///
/// Declaration order is the order the sections render in, most demanding first.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    ToSchema,
    EnumIter,
    IntoStaticStr,
    VariantNames,
)]
pub enum WarningRemedy {
    /// A credential or a scan setting has to change, and both live in Scanopy.
    FixInScanopy,
    /// The device answered and served less than it claims to. Scanopy has no setting for it.
    CheckTheDevice,
    /// A read stopped before it finished. The values already held were kept.
    ClearsOnTheNextScan,
    /// True of the device, or of data that arrived malformed. No scan will change it.
    NothingToDo,
}

impl HasId for WarningRemedy {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for WarningRemedy {
    /// Grey throughout, so the section heading cannot compete with the row it heads. Colour in
    /// this list means severity and only severity; a second coloured vocabulary stacked on top of
    /// it would leave an amber heading over a red row saying two things at once.
    fn color(&self) -> Color {
        Color::Gray
    }

    fn icon(&self) -> Icon {
        match self {
            Self::FixInScanopy => Icon::Wrench,
            Self::CheckTheDevice => Icon::ScanSearch,
            Self::ClearsOnTheNextScan => Icon::RefreshCw,
            Self::NothingToDo => Icon::Info,
        }
    }
}

impl TypeMetadataProvider for WarningRemedy {
    /// The section heading. An instruction, not a category label — it is the whole point of the
    /// grouping that a reader can act on the heading alone.
    fn name(&self) -> &'static str {
        match self {
            Self::FixInScanopy => "Fix in Scanopy",
            Self::CheckTheDevice => "Check the device",
            Self::ClearsOnTheNextScan => "Should clear on the next full scan",
            Self::NothingToDo => "Nothing to do",
        }
    }

    /// The sub-line under the heading, saying why everything below it shares a response.
    fn description(&self) -> &'static str {
        match self {
            Self::FixInScanopy => {
                "A credential or a scan setting has to change. The next scan reads what this one could not."
            }
            Self::CheckTheDevice => {
                "The device answered, but served less than it says it holds. Nothing in Scanopy changes that — the agent's SNMP view or VLAN context has to."
            }
            Self::ClearsOnTheNextScan => {
                "A read stopped before it finished. Previously discovered values were kept, and a complete scan should replace these."
            }
            Self::NothingToDo => {
                "True of the device, or of the data it returned. Rescanning will not change it."
            }
        }
    }
}

impl DiscoveryWarningCode {
    /// The named slots this code's description interpolates.
    ///
    /// Exhaustive, so a new code cannot ship without declaring what fills it.
    pub fn slots(&self) -> &'static [&'static str] {
        match self {
            Self::InterfaceSetCutShort
            | Self::InterfaceDetailsCutShort
            | Self::SnmpCollectedNothing
            | Self::VlanRecordingFailed => &["addresses"],

            // Shaped like the credential family: the integrations identify the statement and the
            // addresses aggregate, so hosts read by the same pair in the same order share a
            // sentence and a different pair gets its own.
            Self::EqualReachIntegrationsMerged => &["addresses", "first", "second"],

            Self::SnmpWalkEntryCap => &["addresses", "groups", "limit"],

            Self::SnmpWalkUnsupported
            | Self::SnmpWalkDesynchronised
            | Self::SnmpWalkPartialDiscarded
            | Self::SnmpWalkPartialRecorded
            | Self::SnmpWalkBridgeMibAbsent
            | Self::SnmpWalkNoAnswer => &["addresses", "groups"],

            Self::ClaimedCountReadCutShort | Self::ClaimedCountUnderRead => {
                &["addresses", "source", "expected", "observed"]
            }
            Self::ClaimedCapabilityReadCutShort | Self::ClaimedCapabilityEmpty => {
                &["addresses", "source", "group"]
            }

            Self::LldpLocalPortDropped | Self::LldpLocalPortDroppedReadCutShort => {
                &["addresses", "dropped", "total"]
            }
            Self::LldpLocalPortMisplaced => &["addresses", "misplaced"],

            Self::MalformedNeighboursWalkCutShort
            | Self::MalformedNeighboursGhostRows
            | Self::MalformedNeighboursIncompleteRecords
            | Self::MalformedNeighboursUnexpectedType
            | Self::MalformedNeighboursUnreadableIndex => {
                &["addresses", "discarded", "consequence"]
            }

            Self::CredentialTargetNotScanned | Self::CredentialTargetNotResponding => {
                &["credential", "addresses"]
            }
            Self::CredentialGateClosed => &["credential", "addresses", "ports"],
            Self::CredentialRejected
            | Self::CredentialMalformed
            | Self::CredentialTlsFailed
            | Self::CredentialNotThisService
            | Self::CredentialCollectionFailed
            | Self::CredentialCollectionTimedOut
            | Self::CredentialUnreachable
            | Self::CredentialTimedOut => &["credential", "addresses", "detail"],

            Self::ConnectionsWithoutProtocolResponse => &["cidr", "declined", "ports"],

            Self::ScanTimeLimitWithEstimate => &["hours", "hosts_not_scanned", "minutes_remaining"],
            Self::ScanTimeLimit => &["hours", "hosts_not_scanned"],

            // No `examples` slot: the pairs these name are a list, and the UI renders them as one
            // under the sentence rather than joining eight of them into its last clause.
            Self::LldpNeighbourNotFound
            | Self::LldpNeighbourAmbiguous
            | Self::LldpPortNoStrategy
            | Self::LldpPortNotFound
            | Self::LldpPortAmbiguous => &["count"],

            Self::ProvisionalSubnetInferred => &["count"],

            Self::NeighbourResolutionIncomplete => &["budget_seconds", "neighbours"],
            Self::FdbResolutionIncomplete => &["budget_seconds", "interfaces"],

            Self::OutdatedDaemonFormat => &["daemon_version"],

            Self::WarningsTruncated => &["elided"],
            Self::Unknown => &["detail"],
        }
    }

    fn severity(&self) -> Severity {
        match self {
            // Nothing was recorded, or the credential cannot be used as configured.
            Self::SnmpWalkPartialDiscarded
            | Self::VlanRecordingFailed
            | Self::LldpLocalPortDropped
            | Self::LldpLocalPortDroppedReadCutShort
            | Self::MalformedNeighboursWalkCutShort
            | Self::MalformedNeighboursGhostRows
            | Self::MalformedNeighboursIncompleteRecords
            | Self::MalformedNeighboursUnexpectedType
            | Self::MalformedNeighboursUnreadableIndex
            | Self::CredentialTargetNotScanned
            | Self::CredentialTargetNotResponding
            | Self::CredentialGateClosed
            | Self::CredentialRejected
            | Self::CredentialMalformed
            | Self::CredentialTlsFailed
            | Self::CredentialNotThisService
            | Self::CredentialCollectionFailed
            | Self::CredentialCollectionTimedOut
            | Self::CredentialUnreachable
            | Self::CredentialTimedOut
            | Self::ScanTimeLimitWithEstimate
            | Self::ScanTimeLimit => Severity::Lost,

            // Not `Lost`: nothing was lost. Addresses that answered a handshake and nothing else
            // were declined, and on the evidence they hold nothing. What the reader has to decide
            // is whether that is true of their network — which is a decision, not a shortfall.
            Self::ConnectionsWithoutProtocolResponse => Severity::Degraded,

            // Incomplete, or resolved to less than it could have been.
            Self::InterfaceSetCutShort
            | Self::SnmpWalkDesynchronised
            | Self::SnmpWalkPartialRecorded
            | Self::SnmpWalkNoAnswer
            | Self::ClaimedCountReadCutShort
            | Self::ClaimedCountUnderRead
            | Self::ClaimedCapabilityReadCutShort
            | Self::ClaimedCapabilityEmpty
            | Self::LldpLocalPortMisplaced
            | Self::SnmpCollectedNothing
            | Self::LldpNeighbourNotFound
            | Self::LldpNeighbourAmbiguous
            | Self::LldpPortNoStrategy
            | Self::LldpPortNotFound
            | Self::LldpPortAmbiguous
            | Self::WarningsTruncated => Severity::Degraded,

            // True of the device, and no scan will change it.
            Self::InterfaceDetailsCutShort
            | Self::SnmpWalkEntryCap
            | Self::SnmpWalkUnsupported
            | Self::SnmpWalkBridgeMibAbsent
            // A range Scanopy proposes, not a fault: the segment is probably real and the operator
            // is being asked to confirm it, which is a different thing from something going wrong.
            | Self::ProvisionalSubnetInferred
            // The old format was translated, so this scan lost nothing. It reports a daemon
            // that will lose something once the compatibility window closes.
            | Self::OutdatedDaemonFormat
            // Running two full-ifTable integrations against one device is supported. Nothing was
            // lost that the configuration did not choose.
            | Self::EqualReachIntegrationsMerged
            | Self::Unknown => Severity::Informational,
            // Links are missing that the device did advertise, which is data loss for this scan
            // rather than something for the operator to confirm.
            Self::NeighbourResolutionIncomplete
            | Self::FdbResolutionIncomplete => Severity::Degraded,
        }
    }

    /// What this code asks of the person reading it.
    ///
    /// Exhaustive, so a new code cannot ship without someone deciding whether it is work for the
    /// reader — which is the question the warning list is organised around and the one the old
    /// flat list never answered.
    fn remedy(&self) -> WarningRemedy {
        match self {
            // The credential family stays whole. Several of these sentences straddle Scanopy and
            // the far end ("check the address and port, and that the service is listening"), and
            // splitting them on where the fault lies would be a guess the diagnostic cannot
            // support. What they share is certain: a person has to look at that credential, and
            // the credential is a Scanopy record.
            Self::CredentialTargetNotScanned
            | Self::CredentialTargetNotResponding
            | Self::CredentialGateClosed
            | Self::CredentialRejected
            | Self::CredentialMalformed
            | Self::CredentialTlsFailed
            | Self::CredentialNotThisService
            | Self::CredentialCollectionFailed
            | Self::CredentialCollectionTimedOut
            | Self::CredentialUnreachable
            | Self::CredentialTimedOut
            // Scan settings: the duration to raise, or the coverage to narrow, or — for the
            // middlebox case — the "Trust Port-Only Detections" opt-in, once the reader has
            // decided the addresses really are empty.
            | Self::ConnectionsWithoutProtocolResponse
            | Self::ScanTimeLimitWithEstimate
            | Self::ScanTimeLimit
            | Self::WarningsTruncated
            // The one warning with a resolution flow of its own, and the one that repeats on
            // every scan until someone answers it.
            | Self::ProvisionalSubnetInferred
            // Duplicate host records are a Scanopy-side data problem, and consolidating them is a
            // shipped action.
            | Self::LldpNeighbourAmbiguous
            // Narrowing what one scan covers is the lever that brings the pass back inside budget.
            | Self::NeighbourResolutionIncomplete
            | Self::FdbResolutionIncomplete
            // Upgrading the daemon is the fix, and the daemon is Scanopy's own component.
            | Self::OutdatedDaemonFormat => WarningRemedy::FixInScanopy,

            // The device says one thing and serves another. No Scanopy setting reaches these:
            // what has to change is the agent's view of its own tables.
            Self::ClaimedCapabilityEmpty
            | Self::ClaimedCountUnderRead
            | Self::SnmpCollectedNothing => WarningRemedy::CheckTheDevice,

            // A read that stopped early or a write that failed once. Their own sentences already
            // say the values held were kept.
            Self::InterfaceSetCutShort
            | Self::InterfaceDetailsCutShort
            | Self::SnmpWalkDesynchronised
            | Self::SnmpWalkNoAnswer
            | Self::SnmpWalkPartialDiscarded
            | Self::SnmpWalkPartialRecorded
            | Self::ClaimedCountReadCutShort
            | Self::ClaimedCapabilityReadCutShort
            | Self::LldpLocalPortDroppedReadCutShort
            | Self::MalformedNeighboursWalkCutShort
            | Self::VlanRecordingFailed => WarningRemedy::ClearsOnTheNextScan,

            // Facts about a device, or about data that arrived malformed and stays that way.
            Self::SnmpWalkUnsupported
            | Self::SnmpWalkEntryCap
            | Self::SnmpWalkBridgeMibAbsent
            | Self::LldpLocalPortDropped
            | Self::LldpLocalPortMisplaced
            | Self::MalformedNeighboursGhostRows
            | Self::MalformedNeighboursIncompleteRecords
            | Self::MalformedNeighboursUnexpectedType
            | Self::MalformedNeighboursUnreadableIndex
            | Self::LldpNeighbourNotFound
            | Self::LldpPortNotFound
            | Self::LldpPortAmbiguous
            | Self::LldpPortNoStrategy
            // A supported configuration, reported so a missing link can be traced to the merge.
            | Self::EqualReachIntegrationsMerged
            // Carries whatever a newer binary sent, so there is nothing here to classify.
            | Self::Unknown => WarningRemedy::NothingToDo,
        }
    }
}

impl HasId for DiscoveryWarningCode {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for DiscoveryWarningCode {
    fn color(&self) -> Color {
        match self.severity() {
            Severity::Lost => Color::Red,
            Severity::Degraded => Color::Amber,
            Severity::Informational => Color::Gray,
        }
    }

    fn icon(&self) -> Icon {
        match self.severity() {
            Severity::Lost => Icon::OctagonAlert,
            Severity::Degraded => Icon::TriangleAlert,
            Severity::Informational => Icon::Info,
        }
    }
}

impl TypeMetadataProvider for DiscoveryWarningCode {
    fn name(&self) -> &'static str {
        match self {
            Self::InterfaceSetCutShort => "Interface list cut short",
            Self::InterfaceDetailsCutShort => "Interface details cut short",
            Self::SnmpWalkEntryCap => "Table larger than one scan reads",
            Self::SnmpWalkUnsupported => "Not implemented over SNMP",
            Self::SnmpWalkDesynchronised => "Agent answered out of step",
            Self::SnmpWalkPartialDiscarded => "Partial read discarded",
            Self::SnmpWalkPartialRecorded => "Partial read recorded",
            Self::SnmpWalkBridgeMibAbsent => "Bridge MIB not served",
            Self::SnmpWalkNoAnswer => "No answer for a table",
            Self::ClaimedCountReadCutShort => "Claimed count, read cut short",
            Self::ClaimedCountUnderRead => "Fewer rows than the device claims",
            Self::ClaimedCapabilityReadCutShort => "Claimed capability, read cut short",
            Self::ClaimedCapabilityEmpty => "Claimed capability, nothing served",
            Self::LldpLocalPortDropped => "LLDP neighbours discarded",
            Self::LldpLocalPortDroppedReadCutShort => "LLDP port table read cut short",
            Self::LldpLocalPortMisplaced => "LLDP neighbours possibly misplaced",
            Self::MalformedNeighboursWalkCutShort => "Neighbour identifiers cut short",
            Self::MalformedNeighboursGhostRows => "Neighbour rows without identifiers",
            Self::MalformedNeighboursIncompleteRecords => "Neighbours listed without identifiers",
            Self::MalformedNeighboursUnexpectedType => "Neighbour identifier of the wrong type",
            Self::MalformedNeighboursUnreadableIndex => "Neighbour position unreadable",
            Self::SnmpCollectedNothing => "SNMP answered but returned nothing",
            Self::VlanRecordingFailed => "VLANs could not be saved",
            Self::EqualReachIntegrationsMerged => "Interfaces merged from two integrations",
            Self::CredentialTargetNotScanned => "Credential target outside the scan",
            Self::CredentialTargetNotResponding => "Credential target did not respond",
            Self::CredentialGateClosed => "Credential port not open",
            Self::CredentialRejected => "Credential refused",
            Self::CredentialMalformed => "Credential incomplete",
            Self::CredentialTlsFailed => "TLS negotiation failed",
            Self::CredentialNotThisService => "Not the expected service",
            Self::CredentialCollectionFailed => "Collection failed after authenticating",
            Self::CredentialCollectionTimedOut => "Collection timed out after authenticating",
            Self::CredentialUnreachable => "Credential target unreachable",
            Self::CredentialTimedOut => "Credential attempt timed out",
            Self::ConnectionsWithoutProtocolResponse => "Handshakes with nothing behind them",
            Self::ScanTimeLimitWithEstimate => "Scan hit its time limit",
            Self::ScanTimeLimit => "Scan hit its time limit",
            Self::LldpNeighbourNotFound => "Neighbour device not discovered",
            Self::LldpNeighbourAmbiguous => "Neighbour device identifier not unique",
            Self::LldpPortNoStrategy => "No lookup for the advertised port id",
            Self::LldpPortNotFound => "Advertised port not found",
            Self::LldpPortAmbiguous => "Advertised port not unique",
            Self::ProvisionalSubnetInferred => "Address range assumed, please confirm",
            Self::NeighbourResolutionIncomplete => "Link resolution did not finish",
            Self::FdbResolutionIncomplete => "FDB link resolution did not finish",
            Self::OutdatedDaemonFormat => "Daemon is outdated",
            Self::WarningsTruncated => "Some warnings not recorded",
            Self::Unknown => "Warning from another version",
        }
    }

    /// The sentence, with `{named}` slots the UI fills. Every slot here must appear in
    /// [`DiscoveryWarningCode::slots`] and vice versa — the test below enforces it.
    fn description(&self) -> &'static str {
        match self {
            Self::InterfaceSetCutShort => {
                "{addresses} stopped responding partway through the SNMP interface list, so some interfaces are missing."
            }
            Self::InterfaceDetailsCutShort => {
                "{addresses} returned every SNMP interface but stopped while reading their details, so some descriptions or speeds may be blank."
            }
            Self::SnmpWalkEntryCap => {
                "{addresses} has more {groups} than one scan reads — collection stops at {limit} entries per table, so the rest were not read. The data recorded is correct as far as it goes."
            }
            Self::SnmpWalkUnsupported => {
                "{addresses} does not implement {groups} over SNMP, so it cannot be read from the device at all. Previously discovered values were kept."
            }
            Self::SnmpWalkDesynchronised => {
                "{addresses} answered out of step with what was asked for {groups}, which usually means the agent is under load. Previously discovered values were kept and refresh on the next complete scan."
            }
            Self::SnmpWalkPartialDiscarded => {
                "{addresses} did not finish reporting {groups}, so what it did answer was not recorded — a partial read cannot tell a value that has gone from one that was not reached. Previously discovered values were kept, and refresh on the next complete scan."
            }
            Self::SnmpWalkPartialRecorded => {
                "{addresses} did not finish reporting {groups}, so what it read was recorded and the rest refreshes on the next complete scan."
            }
            Self::SnmpWalkBridgeMibAbsent => {
                "{addresses} did not answer for {groups}, which these switches commonly do not implement. Their MAC-address-table and VLAN membership cannot be read over SNMP; a UniFi controller integration reports the same data where one manages the device."
            }
            Self::SnmpWalkNoAnswer => {
                "{addresses} returned no {groups} data at all — the device stopped answering rather than reporting that it has none. Previously discovered values were kept rather than overwritten and refresh on the next complete scan."
            }
            Self::ClaimedCountReadCutShort => {
                "{addresses} reported {source} as {expected}, and the read ended at {observed} without finishing. That is how much of the table is missing — the incomplete-walk line for each device says why it ended. What was read is recorded."
            }
            Self::ClaimedCountUnderRead => {
                "{addresses} reported {source} as {expected}, but only {observed} could be read. What was read is recorded; a device may be misreporting its own count, or may be declining to serve the rest of the table to this credential."
            }
            Self::ClaimedCapabilityReadCutShort => {
                "{addresses} advertised {source}, and the read of that {group} ended without returning any. The incomplete-walk line for each device says why it ended; what a device advertises is why this is worth reading again rather than treating as empty."
            }
            Self::ClaimedCapabilityEmpty => {
                "{addresses} advertised {source}, but returned no {group} at all. A device that says it does this and then reports none of it is usually restricting what the credential may see — an SNMP view, or a VLAN context the query has to name."
            }
            Self::LldpLocalPortDropped => {
                "{addresses} reported LLDP neighbours on a local port that matches no interface on the device ({dropped} of {total}), so they are discarded and draw no links — those devices will look as though they have no LLDP neighbours. This usually means the switch numbers its LLDP ports separately from its interfaces, or did not answer for its LLDP port table."
            }
            Self::LldpLocalPortDroppedReadCutShort => {
                "{addresses} reported LLDP neighbours on a local port that matches no interface on the device ({dropped} of {total}), so they are discarded and draw no links — those devices will look as though they have no LLDP neighbours. The read of their own port numbering did not finish, so the numbering could not be matched up; the incomplete-walk line for each device says which table stopped and why. A complete scan may place these neighbours."
            }
            Self::LldpLocalPortMisplaced => {
                "{addresses} reported LLDP neighbours whose local port could not be identified but does match an interface number ({misplaced} in total), so they are drawn against a port that may not be the right one."
            }
            Self::MalformedNeighboursWalkCutShort => {
                "{addresses} reported neighbour records without the identifier needed to match the far end ({discarded} in total), so {consequence}. The read of the column identifying the far end stopped before its end, so these records may come back on the next complete scan."
            }
            Self::MalformedNeighboursGhostRows => {
                "{addresses} reported neighbour records without the identifier needed to match the far end ({discarded} in total), so {consequence}. The rows appeared only in the columns describing each neighbour, never in the one identifying it, so there was no identifier to lose. Rescanning will not change this."
            }
            Self::MalformedNeighboursIncompleteRecords => {
                "{addresses} reported neighbour records without the identifier needed to match the far end ({discarded} in total), so {consequence}. These neighbours were listed and then no identifier was supplied for them. The read finished, so rescanning will not change this."
            }
            Self::MalformedNeighboursUnexpectedType => {
                "{addresses} reported neighbour records without the identifier needed to match the far end ({discarded} in total), so {consequence}. The identifying column came back with a value of a type it cannot hold. Rescanning will not change this."
            }
            Self::MalformedNeighboursUnreadableIndex => {
                "{addresses} reported neighbour records without the identifier needed to match the far end ({discarded} in total), so {consequence}. Their position in the neighbour table could not be read, so they could not be tied to a local port. Rescanning will not change this."
            }
            Self::SnmpCollectedNothing => {
                "{addresses} answered SNMP but returned no interfaces, neighbours, addresses or forwarding data at all. The credential is working, so this is either a device that implements nothing beyond its system description, or one whose tables could not be read — the daemon log records which for each table."
            }
            Self::VlanRecordingFailed => {
                "The VLANs reported by {addresses} could not be saved, so VLAN membership is missing from their interfaces. The devices answered correctly — this is a failure recording the result, and the daemon log has the underlying error."
            }
            Self::EqualReachIntegrationsMerged => {
                "{addresses} answered both {first} and {second}, and both integrations reported a full interface table. Scanopy merged the two sets. On a port both describe, {first} answered first, so Scanopy used its row and its neighbours. Running both is supported."
            }
            Self::CredentialTargetNotScanned => {
                "The {credential} credential for {addresses} was never contacted, because no subnet this scan covers reaches there — add the subnet to the discovery, or move the credential to a host inside it."
            }
            Self::CredentialTargetNotResponding => {
                "The {credential} credential for {addresses} was not tried, because nothing answered there during the scan — check the address is right and the host is online."
            }
            Self::CredentialGateClosed => {
                "The {credential} credential for {addresses} was not tried, because port {ports} was not open there — check the port configured on the credential."
            }
            Self::CredentialRejected => {
                "The {credential} credential for {addresses} was refused — check the username, password or community string. ({detail})"
            }
            Self::CredentialMalformed => {
                "The {credential} credential for {addresses} is incomplete and could not be used — re-enter it. ({detail})"
            }
            Self::CredentialTlsFailed => {
                "The {credential} credential for {addresses} could not negotiate TLS — if the appliance serves a self-signed certificate, turn on \"accept invalid certificates\" in the daemon's scan settings. ({detail})"
            }
            Self::CredentialNotThisService => {
                "The {credential} credential for {addresses} reached something that is not the expected service — check the port on the credential. ({detail})"
            }
            Self::CredentialCollectionFailed => {
                "The {credential} credential for {addresses} authenticated and then failed while collecting, so that data is missing rather than out of date. ({detail})"
            }
            Self::CredentialCollectionTimedOut => {
                "The {credential} credential for {addresses} authenticated and then ran out of time before it finished collecting — rescan separately, or narrow what the scan covers. ({detail})"
            }
            Self::CredentialUnreachable => {
                "The {credential} credential for {addresses} could not be reached — check the address, port and that the service is listening. ({detail})"
            }
            Self::CredentialTimedOut => {
                "The {credential} credential for {addresses} timed out before anything answered — check the address and port, and that the service is listening rather than dropping the connection. ({detail})"
            }
            Self::ConnectionsWithoutProtocolResponse => {
                "{declined} address(es) in {cidr} completed a TCP connection on {ports} and then answered nothing, so no host was recorded at any of them. Something in the path — a firewall session helper, a load balancer, a scrubbing appliance — answers for addresses that hold no device, and a bare handshake cannot tell one from the other. If those addresses really do hold devices, scan the subnet from a daemon with an interface on it; if they are empty, nothing here needs fixing."
            }
            Self::ScanTimeLimitWithEstimate => {
                "Scan hit its time limit ({hours}h) — {hosts_not_scanned} host(s) not scanned (~{minutes_remaining} min of estimated work remaining). Raise Max Discovery Duration or rescan."
            }
            Self::ScanTimeLimit => {
                "Scan hit its time limit ({hours}h) — {hosts_not_scanned} host(s) not scanned. Raise Max Discovery Duration or rescan."
            }
            Self::LldpNeighbourNotFound => {
                "LLDP/CDP neighbours name devices this network has not discovered ({count} in total), so they draw no links. This is expected where the far end is an endpoint or unmanaged device; a device that should have been scanned means the identifier it advertises is not one this network holds."
            }
            Self::LldpNeighbourAmbiguous => {
                "LLDP/CDP neighbours advertise an identifier that several hosts on this network hold ({count} in total), so none of them can be picked and no link is drawn. This is usually duplicate records for one device rather than a device that was missed — consolidate the duplicates and the link resolves."
            }
            Self::LldpPortNoStrategy => {
                "LLDP/CDP neighbours resolved to a device but advertise a port id of a subtype there is no lookup for ({count} in total), so Physical Topology draws a dashed device-level link instead of a port-to-port one."
            }
            Self::LldpPortNotFound => {
                "LLDP/CDP neighbours resolved to a device but no port on it matches the advertised port id ({count} in total), so Physical Topology draws a dashed device-level link instead of a port-to-port one."
            }
            Self::LldpPortAmbiguous => {
                "LLDP/CDP neighbours resolved to a device but several of its ports match the advertised port id ({count} in total), so it identifies none and Physical Topology draws a dashed device-level link instead of a port-to-port one."
            }
            Self::ProvisionalSubnetInferred => {
                "{count} subnet(s) on this network have a range Scanopy assumed rather than read, because devices reported addresses in them that nothing scanned holds. Nothing advertises a netmask, so the range around an address is a convention — confirm or correct it on the subnet. No daemon has an interface on these ranges, and they are reported on every scan until confirmed."
            }
            Self::NeighbourResolutionIncomplete => {
                "Matching LLDP/CDP neighbours to the devices and ports they name was stopped after {budget_seconds}s, with {neighbours} interface(s) advertising a neighbour on this network. Physical Topology is missing links this scan would otherwise have drawn; the next scan retries from scratch. Narrow what the scan covers, or split the network across daemons, if it keeps happening."
            }
            Self::FdbResolutionIncomplete => {
                "Matching single-MAC forwarding-table entries to the devices they name was stopped after {budget_seconds}s, with {interfaces} interface(s) still carrying one. Physical Topology is missing links this scan would otherwise have drawn; the next scan retries from scratch. Narrow what the scan covers, or split the network across daemons, if it keeps happening."
            }
            Self::OutdatedDaemonFormat => {
                "The daemon that ran this scan is on {daemon_version}. Upgrade it."
            }
            Self::WarningsTruncated => {
                "{elided} further warnings from this scan were not recorded, because it produced more than the scan record holds. Narrow what the scan covers to see the rest."
            }
            Self::Unknown => "{detail}",
        }
    }

    /// Publishes the slot contract to the UI, which builds its parameters from the same list.
    /// The rung this code sits on, which is the section the UI files it under.
    ///
    /// `category` rather than a new field: `TypeMetadata` already has one, documented as the group
    /// a type is listed under, and warning codes were the registry leaving it null.
    fn category(&self) -> &'static str {
        self.remedy().id()
    }

    fn metadata(&self) -> serde_json::Value {
        json!({ "slots": self.slots() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::types::metadata::TypeMetadataProvider;
    use std::collections::BTreeSet;
    use strum::IntoEnumIterator;

    /// Slots in a description have to match the slots the code declares, or the UI passes
    /// parameters paraglide does not expect and the sentence renders with holes in it.
    ///
    /// The pairing is otherwise pure author discipline across two files and a language boundary:
    /// the template here, the parameters the TypeScript renderer builds, and the paraglide message
    /// signature generated from this very string. Checking it here is the cheapest place to fail.
    #[test]
    fn every_description_interpolates_exactly_the_slots_it_declares() {
        let mut mismatched = Vec::new();

        for code in DiscoveryWarningCode::iter() {
            let in_template: BTreeSet<String> =
                crate::server::shared::types::metadata::extract_slots(code.description())
                    .into_iter()
                    .collect();
            let declared: BTreeSet<String> =
                code.slots().iter().map(|s| (*s).to_string()).collect();

            if in_template != declared {
                mismatched.push(format!(
                    "  {}\n    template declares: {:?}\n    slots() declares:  {:?}",
                    code.id(),
                    in_template,
                    declared,
                ));
            }
        }

        assert!(
            mismatched.is_empty(),
            "description templates and slots() disagree:\n{}",
            mismatched.join("\n")
        );
    }

    /// A slot value the UI cannot resolve renders as an empty string, so every code has to have
    /// something to say even when it takes no parameters at all.
    #[test]
    fn every_code_has_a_name_and_a_description() {
        for code in DiscoveryWarningCode::iter() {
            assert!(!code.name().is_empty(), "{} has no name", code.id());
            assert!(
                !code.description().is_empty(),
                "{} has no description",
                code.id()
            );
        }
    }
}
