//! Turning accumulated records into coded warnings.
//!
//! One function per record kind, and each is the same shape: decide which code this record is an
//! instance of, then hand the rest of the record over as that occurrence's detail. What used to be
//! a renderer's `match` over English sentences is now a `match` over codes — the arms are the
//! same, which is where the codes came from.
//!
//! Nothing here aggregates. A record about one device becomes a warning about one device, and the
//! grouping that used to happen before the wire ("these six switches all did X") happens in the UI
//! instead, where `Intl.ListFormat` can join the addresses in the reader's language. The old
//! summing is why `collected` counts never reached anyone and why a credential's diagnostic was
//! only ever shown for the first address in a batch.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::net::IpAddr;

use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;

use strum::EnumCount;

use super::{
    AttemptOutcome, ContradictedClaim, CredentialIssue, CredentialIssueReason, DeviceClaim,
    EqualReachIntegrations, IncompleteInterfaceWalk, IncompleteSnmpWalk, LocalPortPlacementReason,
    MalformedNeighbourReason, MalformedNeighbours, ShortfallReason, SnmpCollectedNothing,
    UnresolvedLldpPorts, VlanRecordingFailed,
};
use crate::daemon::discovery::types::warnings::{
    CredentialAttempt, DiscoveryWarning, MalformedNeighbourConsequence,
    MalformedNeighbours as MalformedNeighboursDetail,
};

/// A count that has to survive as a `u32` on the wire. Saturating rather than wrapping: a
/// nonsense-large count should read as "very many", never as a small number.
fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Interface walks split by what fell short, because the two mean different things: a truncated
/// interface *set* means interfaces are genuinely missing, a truncated attribute column only means
/// some descriptions or speeds are blank. Reporting the second as possible data loss sends people
/// hunting for interfaces that were never absent.
pub fn warn_incomplete_interface_walks(
    records: &[IncompleteInterfaceWalk],
) -> Vec<DiscoveryWarning> {
    records
        .iter()
        .map(|record| {
            let address = record.ip;
            let collected = count(record.collected);
            if record.set_complete {
                DiscoveryWarning::InterfaceDetailsCutShort { address, collected }
            } else {
                DiscoveryWarning::InterfaceSetCutShort { address, collected }
            }
        })
        .collect()
}

/// The seven ways a walk can come up short.
///
/// Keyed exactly as the renderer this replaces was — on `(reason, returned_any, group)` — because
/// that key *is* the distinction an operator cares about, and it is what makes the metric able to
/// tell "the device does not implement this" from "the device stopped answering".
pub fn warn_incomplete_snmp_walks(records: &[IncompleteSnmpWalk]) -> Vec<DiscoveryWarning> {
    records
        .iter()
        .map(|record| {
            let address = record.ip;
            let group = record.group;
            match record.reason {
                Some(ShortfallReason::EntryCap { limit }) => DiscoveryWarning::SnmpWalkEntryCap {
                    address,
                    group,
                    limit: count(limit),
                },
                Some(ShortfallReason::Unsupported) => {
                    DiscoveryWarning::SnmpWalkUnsupported { address, group }
                }
                // Ahead of the `returned_any` arms deliberately: a desynchronised agent is a
                // statement about the agent, and it holds whether or not rows came back.
                Some(ShortfallReason::Desynchronised) => {
                    DiscoveryWarning::SnmpWalkDesynchronised { address, group }
                }
                None | Some(ShortfallReason::NoAnswer) => {
                    if record.returned_any {
                        if group.partial_read_is_discarded() {
                            DiscoveryWarning::SnmpWalkPartialDiscarded { address, group }
                        } else {
                            DiscoveryWarning::SnmpWalkPartialRecorded { address, group }
                        }
                    } else if group.absence_means_unsupported() {
                        DiscoveryWarning::SnmpWalkBridgeMibAbsent { address, group }
                    } else {
                        DiscoveryWarning::SnmpWalkNoAnswer { address, group }
                    }
                }
            }
        })
        .collect()
}

/// A device disagreeing with itself, split by what it claimed and whether the read finished.
///
/// The `cut_short` half matters: a device whose walk was cut off has not declined to serve
/// anything and is not misreporting itself, and saying so beside a line that says it stopped
/// responding leaves the reader to reconcile two accounts of one device.
pub fn warn_contradicted_claims(records: &[ContradictedClaim]) -> Vec<DiscoveryWarning> {
    records
        .iter()
        .map(|record| {
            let address = record.ip;
            let group = record.group;
            let observed = count(record.observed);
            let cut_short = record
                .reason
                .is_some_and(ShortfallReason::read_was_cut_short);

            match record.claim {
                DeviceClaim::Count { source, expected } => {
                    let expected = count(expected);
                    if cut_short {
                        DiscoveryWarning::ClaimedCountReadCutShort {
                            address,
                            group,
                            source,
                            expected,
                            observed,
                        }
                    } else {
                        DiscoveryWarning::ClaimedCountUnderRead {
                            address,
                            group,
                            source,
                            expected,
                            observed,
                        }
                    }
                }
                DeviceClaim::Implements { source } => {
                    if cut_short {
                        DiscoveryWarning::ClaimedCapabilityReadCutShort {
                            address,
                            group,
                            source,
                        }
                    } else {
                        DiscoveryWarning::ClaimedCapabilityEmpty {
                            address,
                            group,
                            source,
                        }
                    }
                }
            }
        })
        .collect()
}

/// Up to two warnings per device, because a device can lose neighbours both ways at once.
///
/// The misplaced count deliberately excludes the dropped ones: those are the other warning, and
/// reporting them twice under two different consequences reads as two problems.
pub fn warn_unresolved_lldp_ports(records: &[UnresolvedLldpPorts]) -> Vec<DiscoveryWarning> {
    let mut warnings = Vec::new();

    for record in records {
        if record.dropped > 0 {
            let address = record.ip;
            let dropped = count(record.dropped);
            let total = count(record.total);
            // A `match` rather than a flag, so a third reason cannot be added without deciding how
            // it reads. The two sentences differ on whether a rescan can help, which is the whole
            // reason they are separate codes.
            warnings.push(match record.reason {
                LocalPortPlacementReason::ReadsComplete => DiscoveryWarning::LldpLocalPortDropped {
                    address,
                    dropped,
                    total,
                },
                LocalPortPlacementReason::ReadCutShort => {
                    DiscoveryWarning::LldpLocalPortDroppedReadCutShort {
                        address,
                        dropped,
                        total,
                    }
                }
            });
        }
        let misplaced = record.unresolved.saturating_sub(record.dropped);
        if misplaced > 0 {
            warnings.push(DiscoveryWarning::LldpLocalPortMisplaced {
                address: record.ip,
                misplaced: count(misplaced),
            });
        }
    }

    warnings
}

/// One warning per device and cause. The causes differ on whether a rescan helps, which is why
/// each is its own code: a single code covering all of them has to pick one answer and be wrong
/// for the rest.
pub fn warn_malformed_neighbours(records: &[MalformedNeighbours]) -> Vec<DiscoveryWarning> {
    records
        .iter()
        .filter(|record| record.discarded > 0)
        .map(|record| {
            let detail = MalformedNeighboursDetail {
                address: record.ip,
                group: record.group,
                discarded: count(record.discarded),
                kept: count(record.kept),
                consequence: MalformedNeighbourConsequence::from_kept(record.kept),
            };
            match record.reason {
                MalformedNeighbourReason::WalkCutShort => {
                    DiscoveryWarning::MalformedNeighboursWalkCutShort(detail)
                }
                MalformedNeighbourReason::GhostRows => {
                    DiscoveryWarning::MalformedNeighboursGhostRows(detail)
                }
                MalformedNeighbourReason::IncompleteRecords => {
                    DiscoveryWarning::MalformedNeighboursIncompleteRecords(detail)
                }
                MalformedNeighbourReason::UnexpectedType => {
                    DiscoveryWarning::MalformedNeighboursUnexpectedType(detail)
                }
                MalformedNeighbourReason::UnreadableIndex => {
                    DiscoveryWarning::MalformedNeighboursUnreadableIndex(detail)
                }
            }
        })
        .collect()
}

pub fn warn_snmp_collected_nothing(records: &[SnmpCollectedNothing]) -> Vec<DiscoveryWarning> {
    records
        .iter()
        .map(|record| DiscoveryWarning::SnmpCollectedNothing { address: record.ip })
        .collect()
}

pub fn warn_vlan_recording_failures(records: &[VlanRecordingFailed]) -> Vec<DiscoveryWarning> {
    records
        .iter()
        .map(|record| DiscoveryWarning::VlanRecordingFailed { address: record.ip })
        .collect()
}

/// One warning per host, carrying which integration answered first. The order is the substance:
/// it says whose row stands on every port both describe.
pub fn warn_equal_reach_integrations(records: &[EqualReachIntegrations]) -> Vec<DiscoveryWarning> {
    records
        .iter()
        .map(|record| DiscoveryWarning::EqualReachIntegrationsMerged {
            address: record.ip,
            first: record.first,
            second: record.second,
        })
        .collect()
}

/// One warning per credential issue, in the order the outcomes are worth acting on.
///
/// Ordering survives the move to codes because it is what puts the finding an operator can fix
/// above the ones that only describe the network. What does *not* survive is the batching: each
/// issue now carries its own address and its own diagnostic, where the rendered line could only
/// ever quote the first message it found for a whole outcome.
pub fn warn_credential_issues(issues: &[CredentialIssue]) -> Vec<DiscoveryWarning> {
    let mut warnings = Vec::new();

    for issue in issues {
        if issue.reason == CredentialIssueReason::TargetNotScanned {
            warnings.push(DiscoveryWarning::CredentialTargetNotScanned {
                address: issue.ip,
                integration: issue.integration,
                credential_id: issue.credential_id,
            });
        }
    }
    for issue in issues {
        if issue.reason == CredentialIssueReason::TargetNotResponding {
            warnings.push(DiscoveryWarning::CredentialTargetNotResponding {
                address: issue.ip,
                integration: issue.integration,
                credential_id: issue.credential_id,
            });
        }
    }
    for issue in issues {
        if let CredentialIssueReason::GateClosed { ports } = &issue.reason {
            warnings.push(DiscoveryWarning::CredentialGateClosed {
                address: issue.ip,
                integration: issue.integration,
                credential_id: issue.credential_id,
                ports: ports
                    .iter()
                    .map(|p| p.number())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            });
        }
    }

    // Which address stands for a collapsed group. It is arbitrary by definition — the credential
    // is broken identically everywhere — so it is the lowest rather than whichever the scan
    // happened to reach first. Scan order varies between runs, and taking the first made one
    // unchanged broken credential name a different host every time (192.168.7.73, then
    // 192.168.4.187, then 192.168.4.29), which reads as a new problem and makes two runs
    // impossible to compare.
    let mut stands_for: HashMap<(CredentialQueryPayloadDiscriminants, &str), IpAddr> =
        HashMap::new();
    for issue in issues {
        let Some(outcome) = attempt_outcome(&issue.reason) else {
            continue;
        };
        if !is_one_finding_per_credential(outcome) {
            continue;
        }
        stands_for
            .entry((issue.integration, attempt_message(&issue.reason)))
            .and_modify(|ip| *ip = (*ip).min(issue.ip))
            .or_insert(issue.ip);
    }

    // One finding per broken credential, for the outcome where the address is not part of the
    // problem. See `is_one_finding_per_credential`.
    let mut reported_once: HashSet<(CredentialQueryPayloadDiscriminants, &str)> = HashSet::new();
    for outcome in ATTEMPT_ORDER {
        for issue in issues {
            if attempt_outcome(&issue.reason) != Some(outcome) {
                continue;
            }
            if already_covered_by_address_line(issues, issue) {
                continue;
            }
            if is_one_finding_per_credential(outcome) {
                let message = attempt_message(&issue.reason);
                if stands_for.get(&(issue.integration, message)) != Some(&issue.ip) {
                    continue;
                }
                if !reported_once.insert((issue.integration, message)) {
                    continue;
                }
            }
            let detail = match &issue.reason {
                CredentialIssueReason::Attempted { message, .. } if !message.is_empty() => {
                    Some(message.clone())
                }
                _ => None,
            };
            let attempt = CredentialAttempt {
                address: issue.ip,
                integration: issue.integration,
                detail,
                credential_id: issue.credential_id,
            };
            let Some(warning) = warning_for_outcome(outcome, attempt) else {
                continue;
            };
            warnings.push(warning);
        }
    }

    warnings
}

/// The code for one attempt outcome, or `None` for an outcome that is not a finding.
///
/// A `match` rather than a lookup table, so a new outcome cannot be added without deciding how it
/// reads. As a table this compiled fine with a variant missing and simply never mentioned it — the
/// failure mode being that an operator hits a problem the product has a name for and is told
/// nothing at all.
fn warning_for_outcome(
    outcome: AttemptOutcome,
    attempt: CredentialAttempt,
) -> Option<DiscoveryWarning> {
    Some(match outcome {
        AttemptOutcome::Rejected => DiscoveryWarning::CredentialRejected(attempt),
        AttemptOutcome::Malformed => DiscoveryWarning::CredentialMalformed(attempt),
        AttemptOutcome::TlsFailed => DiscoveryWarning::CredentialTlsFailed(attempt),
        AttemptOutcome::NotThisService => DiscoveryWarning::CredentialNotThisService(attempt),
        AttemptOutcome::CollectionFailed => DiscoveryWarning::CredentialCollectionFailed(attempt),
        // Deliberately outside the `already_covered_by_address_line` suppression: no address-level
        // warning says this, because the address answered fine. Suppressing it the way `TimedOut`
        // is suppressed is what made a 300s container scan silent.
        AttemptOutcome::CollectionTimedOut => {
            DiscoveryWarning::CredentialCollectionTimedOut(attempt)
        }
        AttemptOutcome::Unreachable => DiscoveryWarning::CredentialUnreachable(attempt),
        AttemptOutcome::TimedOut => DiscoveryWarning::CredentialTimedOut(attempt),
        // The user stopped the scan. Not a finding.
        AttemptOutcome::Cancelled => return None,
    })
}

/// The order the outcomes are emitted in: most actionable first, so the warning an operator can do
/// something about is not buried under the ones describing the network.
///
/// Fixed-length rather than a slice, so a new [`AttemptOutcome`] that is not listed here is an
/// array-length compile error. As a slice this compiled fine with a variant missing and simply
/// never emitted it.
const ATTEMPT_ORDER: [AttemptOutcome; AttemptOutcome::COUNT] = [
    AttemptOutcome::Rejected,
    AttemptOutcome::Malformed,
    AttemptOutcome::TlsFailed,
    AttemptOutcome::NotThisService,
    AttemptOutcome::CollectionFailed,
    AttemptOutcome::CollectionTimedOut,
    AttemptOutcome::Unreachable,
    AttemptOutcome::TimedOut,
    AttemptOutcome::Cancelled,
];

/// Whether this outcome describes the credential rather than the address it was tried at, and so
/// is one finding however many hosts met it.
///
/// [`AttemptOutcome::Malformed`] alone. Every other outcome is the *device's* answer — refused,
/// silent, not this service — and those are deliberately kept per-address, because the diagnostic
/// belongs to that host and merging them re-does the batching the per-occurrence records exist to
/// undo. A credential that could not be read has no per-host component at all: the same message,
/// from the same broken field, at every address in the subnet, fixed by one edit. Undeduped, a /24
/// sweep put two hundred identical bullets in front of the operator (GH #668).
///
/// A `match` rather than an `==` so a new outcome has to be considered rather than defaulted.
fn is_one_finding_per_credential(outcome: AttemptOutcome) -> bool {
    match outcome {
        AttemptOutcome::Malformed => true,
        AttemptOutcome::Rejected
        | AttemptOutcome::Unreachable
        | AttemptOutcome::TimedOut
        | AttemptOutcome::NotThisService
        | AttemptOutcome::TlsFailed
        | AttemptOutcome::CollectionFailed
        | AttemptOutcome::CollectionTimedOut
        | AttemptOutcome::Cancelled => false,
    }
}

/// Whether an address-level warning in the same batch already says what this one would.
///
/// "Nothing answered at 10.0.0.5" and "the SNMP credential for 10.0.0.5 could not be reached" are
/// the same fact, and on a sweep the first is reported per address already. So the second is
/// dropped — but *only* when the first is actually present. Deciding this by outcome alone, as
/// this used to, silently swallowed every unreachable-credential report from the daemon-host
/// phase, where no address-level warning exists because 127.0.0.1 is always up.
fn already_covered_by_address_line(issues: &[CredentialIssue], issue: &CredentialIssue) -> bool {
    if !matches!(
        attempt_outcome(&issue.reason),
        Some(AttemptOutcome::Unreachable | AttemptOutcome::TimedOut)
    ) {
        return false;
    }
    issues.iter().any(|other| {
        other.ip == issue.ip
            && matches!(
                other.reason,
                CredentialIssueReason::TargetNotScanned
                    | CredentialIssueReason::TargetNotResponding
            )
    })
}

fn attempt_outcome(reason: &CredentialIssueReason) -> Option<AttemptOutcome> {
    match reason {
        CredentialIssueReason::Attempted { outcome, .. } => Some(*outcome),
        _ => None,
    }
}

/// The library's diagnostic, which is half of what identifies a collapsed group — two credentials
/// of one integration failing for different reasons stay two findings.
fn attempt_message(reason: &CredentialIssueReason) -> &str {
    match reason {
        CredentialIssueReason::Attempted { message, .. } => message.as_str(),
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::discovery::types::warnings::{
        ClaimSource, DiscoveryWarningCode, SnmpWalkGroup,
    };
    use crate::server::credentials::r#impl::mapping::CredentialQueryPayloadDiscriminants;
    use crate::server::ports::r#impl::base::PortType;
    use std::net::IpAddr;
    use strum::IntoEnumIterator;
    use uuid::Uuid;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    fn codes(warnings: &[DiscoveryWarning]) -> Vec<DiscoveryWarningCode> {
        warnings.iter().map(DiscoveryWarning::code).collect()
    }

    fn walk(
        address: &str,
        group: SnmpWalkGroup,
        returned_any: bool,
        reason: Option<ShortfallReason>,
    ) -> IncompleteSnmpWalk {
        IncompleteSnmpWalk {
            ip: ip(address),
            group,
            returned_any,
            reason,
        }
    }

    /// A walk that returned nothing and one that was truncated are different problems, and the
    /// single phrasing these codes replaced ("stopped responding partway through") described only
    /// the second.
    #[test]
    fn an_empty_walk_is_a_different_code_from_a_truncated_one() {
        let truncated = warn_incomplete_snmp_walks(&[walk(
            "10.0.0.1",
            SnmpWalkGroup::BridgeForwarding,
            true,
            None,
        )]);
        let empty = warn_incomplete_snmp_walks(&[walk(
            "10.0.0.1",
            SnmpWalkGroup::BridgeForwarding,
            false,
            None,
        )]);

        assert_eq!(
            codes(&truncated),
            vec![DiscoveryWarningCode::SnmpWalkPartialDiscarded]
        );
        assert_eq!(codes(&empty), vec![DiscoveryWarningCode::SnmpWalkNoAnswer]);
    }

    /// A partial read of a group whose rows survive is not the same finding as one whose rows are
    /// thrown away: the first keeps what it read, the second contributes nothing however much it
    /// answered (GH #685).
    #[test]
    fn a_partial_read_that_is_kept_is_a_different_code_from_one_that_is_discarded() {
        let discarded = warn_incomplete_snmp_walks(&[walk(
            "10.0.0.1",
            SnmpWalkGroup::BridgeForwarding,
            true,
            None,
        )]);
        let recorded =
            warn_incomplete_snmp_walks(&[walk("10.0.0.1", SnmpWalkGroup::ArpTable, true, None)]);

        assert_eq!(
            codes(&discarded),
            vec![DiscoveryWarningCode::SnmpWalkPartialDiscarded]
        );
        assert_eq!(
            codes(&recorded),
            vec![DiscoveryWarningCode::SnmpWalkPartialRecorded]
        );
    }

    /// The bridge-MIB root, absent rather than truncated, is the "these switches commonly do not
    /// implement it" case. Truncated, it is an ordinary short read — `absence_means_unsupported`
    /// gates on the empty case alone.
    #[test]
    fn a_truncated_bridge_port_walk_is_still_a_truncation() {
        let truncated = warn_incomplete_snmp_walks(&[walk(
            "192.168.210.217",
            SnmpWalkGroup::BridgePortNumbering,
            true,
            None,
        )]);
        let absent = warn_incomplete_snmp_walks(&[walk(
            "192.168.210.217",
            SnmpWalkGroup::BridgePortNumbering,
            false,
            None,
        )]);

        assert_eq!(
            codes(&truncated),
            vec![DiscoveryWarningCode::SnmpWalkPartialRecorded],
            "bridge-port numbering keeps the rows a short read returned, so a truncated one is \
             an ordinary partial read"
        );
        assert_eq!(
            codes(&absent),
            vec![DiscoveryWarningCode::SnmpWalkBridgeMibAbsent]
        );
    }

    /// Our own limit, hit every scan on a device this size. Reporting it as a shortfall sent an
    /// operator looking for a fault on hardware that was answering perfectly, and no amount of
    /// re-scanning would have changed it. The limit rides on the warning because it is the whole
    /// point of the sentence.
    #[test]
    fn hitting_the_entry_cap_carries_the_limit_and_is_not_a_shortfall() {
        let warnings = warn_incomplete_snmp_walks(&[walk(
            "10.0.0.1",
            SnmpWalkGroup::BridgeForwarding,
            true,
            Some(ShortfallReason::EntryCap { limit: 10_000 }),
        )]);

        assert_eq!(
            warnings,
            vec![DiscoveryWarning::SnmpWalkEntryCap {
                address: ip("10.0.0.1"),
                group: SnmpWalkGroup::BridgeForwarding,
                limit: 10_000,
            }]
        );
    }

    /// An agent answering out of step is a statement about the agent, so it outranks the
    /// `returned_any` split: whether rows came back does not change what to do about it.
    #[test]
    fn a_desynchronised_agent_outranks_whether_rows_came_back() {
        for returned_any in [true, false] {
            let warnings = warn_incomplete_snmp_walks(&[walk(
                "10.0.0.1",
                SnmpWalkGroup::Lldp,
                returned_any,
                Some(ShortfallReason::Desynchronised),
            )]);
            assert_eq!(
                codes(&warnings),
                vec![DiscoveryWarningCode::SnmpWalkDesynchronised],
                "returned_any = {returned_any}"
            );
        }
    }

    /// The headline contradiction case, and the one the whole mechanism exists for: a device that
    /// publishes a count, answers with a fraction of it, and until now reported a clean scan.
    /// Both figures travel, because a warning saying only that the count "looks wrong" leaves the
    /// operator nothing to check against their own switch.
    #[test]
    fn a_device_reading_far_below_its_own_count_carries_both_figures() {
        let warnings = warn_contradicted_claims(&[ContradictedClaim {
            ip: ip("192.168.200.151"),
            group: SnmpWalkGroup::Interfaces,
            claim: DeviceClaim::Count {
                source: ClaimSource::IfNumber,
                expected: 23,
            },
            observed: 1,
            reason: None,
        }]);

        assert_eq!(
            warnings,
            vec![DiscoveryWarning::ClaimedCountUnderRead {
                address: ip("192.168.200.151"),
                group: SnmpWalkGroup::Interfaces,
                source: ClaimSource::IfNumber,
                expected: 23,
                observed: 1,
            }]
        );
    }

    /// The defect this branch exists for, found by scanning the sim: three devices whose ifTable
    /// walk was cut off by simulator load each got a warning saying they "may be misreporting"
    /// their count or were "declining to serve" rows — beside one already saying they stopped
    /// responding partway. Two accounts of one device, and the more specific one was wrong.
    ///
    /// The figures still travel: the shortfall knows the read ended but not how much of the table
    /// it missed, because only the device knows that.
    #[test]
    fn a_cut_short_read_is_a_different_code_from_a_completed_short_one() {
        let claim = |reason| ContradictedClaim {
            ip: ip("192.168.7.250"),
            group: SnmpWalkGroup::Interfaces,
            claim: DeviceClaim::Count {
                source: ClaimSource::IfNumber,
                expected: 52,
            },
            observed: 14,
            reason,
        };

        assert_eq!(
            codes(&warn_contradicted_claims(&[claim(Some(
                ShortfallReason::NoAnswer
            ))])),
            vec![DiscoveryWarningCode::ClaimedCountReadCutShort]
        );
        assert_eq!(
            codes(&warn_contradicted_claims(&[claim(None)])),
            vec![DiscoveryWarningCode::ClaimedCountUnderRead]
        );
    }

    /// An unsupported table is not a cut-short read, so it keeps the cause a cut-short one drops:
    /// the device answered everything asked of it and simply has none of this.
    #[test]
    fn an_unsupported_table_is_not_treated_as_a_cut_short_read() {
        let warnings = warn_contradicted_claims(&[ContradictedClaim {
            ip: ip("10.0.0.1"),
            group: SnmpWalkGroup::BridgePortNumbering,
            claim: DeviceClaim::Implements {
                source: ClaimSource::SysServicesBridgeBit,
            },
            observed: 0,
            reason: Some(ShortfallReason::Unsupported),
        }]);

        assert_eq!(
            codes(&warnings),
            vec![DiscoveryWarningCode::ClaimedCapabilityEmpty]
        );
    }

    /// The two LLDP-port outcomes are separate warnings, because the operator can act on one and
    /// can only distrust the other: a discarded neighbour is a link that is not on the map at all,
    /// while a neighbour placed on an unconfirmed port is on it and may be in the wrong place. A
    /// device with both raises both, and the counts partition — reporting a discarded neighbour
    /// under the second heading too would double-count it.
    #[test]
    fn discarded_and_misplaced_neighbours_are_separate_outcomes() {
        let both = warn_unresolved_lldp_ports(&[UnresolvedLldpPorts {
            ip: ip("192.168.7.238"),
            unresolved: 3,
            dropped: 1,
            total: 4,
            reason: LocalPortPlacementReason::ReadsComplete,
        }]);
        assert_eq!(
            both,
            vec![
                DiscoveryWarning::LldpLocalPortDropped {
                    address: ip("192.168.7.238"),
                    dropped: 1,
                    total: 4,
                },
                DiscoveryWarning::LldpLocalPortMisplaced {
                    address: ip("192.168.7.238"),
                    misplaced: 2,
                },
            ]
        );

        let dropped_only = warn_unresolved_lldp_ports(&[UnresolvedLldpPorts {
            ip: ip("192.168.7.238"),
            unresolved: 3,
            dropped: 3,
            total: 4,
            reason: LocalPortPlacementReason::ReadsComplete,
        }]);
        assert_eq!(
            codes(&dropped_only),
            vec![DiscoveryWarningCode::LldpLocalPortDropped],
            "every unplaced neighbour was discarded, so there is no second population"
        );
    }

    /// GH #668. The same loss, told two ways, because the operator can act on one of them.
    ///
    /// The single code these replace asserted a cause — that the switch numbers its LLDP ports
    /// separately from its interfaces — and hedged it with "or did not answer for its LLDP port
    /// table". On a device whose port table demonstrably did not finish, the first half is a
    /// diagnosis of a fault it does not have, and the operator had no way to tell which half
    /// applied to them.
    #[test]
    fn a_placement_lost_to_a_short_read_is_a_different_code_from_one_lost_to_numbering() {
        let dropped = |reason| {
            warn_unresolved_lldp_ports(&[UnresolvedLldpPorts {
                ip: ip("192.168.7.253"),
                unresolved: 0,
                dropped: 2,
                total: 6,
                reason,
            }])
        };

        assert_eq!(
            codes(&dropped(LocalPortPlacementReason::ReadCutShort)),
            vec![DiscoveryWarningCode::LldpLocalPortDroppedReadCutShort],
            "the numbering was never fully read, so it cannot be what the numbering is blamed for"
        );
        assert_eq!(
            codes(&dropped(LocalPortPlacementReason::ReadsComplete)),
            vec![DiscoveryWarningCode::LldpLocalPortDropped],
            "a device that told us everything it has and still does not line up is the case the \
             port-numbering sentence was written for, and keeps it"
        );
    }

    /// A device that placed every neighbour somewhere real is not silent about it: the walk itself
    /// is what the tiers could not confirm, and that is still worth reporting even though nothing
    /// was lost.
    #[test]
    fn a_device_that_dropped_nothing_still_reports_its_unconfirmed_ports() {
        let warnings = warn_unresolved_lldp_ports(&[UnresolvedLldpPorts {
            ip: ip("10.0.0.1"),
            unresolved: 2,
            dropped: 0,
            total: 6,
            reason: LocalPortPlacementReason::ReadsComplete,
        }]);

        assert_eq!(
            warnings,
            vec![DiscoveryWarning::LldpLocalPortMisplaced {
                address: ip("10.0.0.1"),
                misplaced: 2,
            }]
        );
    }

    #[test]
    fn fully_resolved_lldp_ports_are_not_reported() {
        assert!(
            warn_unresolved_lldp_ports(&[UnresolvedLldpPorts {
                ip: ip("10.0.0.1"),
                unresolved: 0,
                dropped: 0,
                total: 6,
                reason: LocalPortPlacementReason::ReadsComplete,
            }])
            .is_empty()
        );
    }

    /// GH #668: a credential that cannot be read is one problem, not one problem per host.
    ///
    /// Every other credential warning carries that address's own diagnostic, which is why they are
    /// rendered one bullet per occurrence. This one cannot: the fault is in the credential, the
    /// message is identical everywhere, and the fix is a single edit. Undeduped, a /24 sweep would
    /// put two hundred copies of "re-enter it" in front of the operator.
    ///
    /// Deduped on the diagnostic as well as the integration, so two *different* unreadable fields
    /// — a missing community file and a missing v3 password file — stay two findings.
    #[test]
    fn an_unreadable_credential_is_reported_once_however_many_hosts_met_it() {
        let at = |address: &str, detail: &str| CredentialIssue {
            integration: CredentialQueryPayloadDiscriminants::Snmp,
            ip: ip(address),
            reason: CredentialIssueReason::Attempted {
                outcome: AttemptOutcome::Malformed,
                message: detail.to_string(),
            },
            credential_id: None,
        };
        let unreadable = "Failed to read community from public for SNMP: No such file or directory";
        let sweep: Vec<CredentialIssue> = (1..=20)
            .map(|host| at(&format!("10.0.0.{host}"), unreadable))
            .collect();

        assert_eq!(
            warn_credential_issues(&sweep).len(),
            1,
            "one broken credential is one finding, whatever the size of the subnet it was tried in"
        );

        let two_faults = vec![
            at("10.0.0.1", unreadable),
            at(
                "10.0.0.2",
                "Failed to read auth_password from /etc/snmp.pw for SNMPv3",
            ),
        ];
        assert_eq!(
            warn_credential_issues(&two_faults).len(),
            2,
            "two different fields the operator has to fix are two findings"
        );
    }

    /// Which address stands for a collapsed group must not depend on the order the scan met them.
    ///
    /// The credential is broken identically everywhere, so the address is arbitrary — but taking
    /// whichever arrived first made one unchanged broken credential name a different host every
    /// run as scan order shifted, which reads as a new problem and makes two runs impossible to
    /// compare. Asserted as the property (same input set, same finding) rather than by pinning a
    /// literal, so it survives any future choice of representative.
    #[test]
    fn a_collapsed_finding_names_the_same_address_whatever_order_it_met_them() {
        let at = |address: &str| CredentialIssue {
            integration: CredentialQueryPayloadDiscriminants::Snmp,
            ip: ip(address),
            reason: CredentialIssueReason::Attempted {
                outcome: AttemptOutcome::Malformed,
                message: "Failed to read community from public".to_string(),
            },
            credential_id: None,
        };
        let addresses = ["192.168.7.73", "192.168.4.187", "192.168.4.29"];

        let forwards = warn_credential_issues(&addresses.map(at));
        let mut reversed = addresses;
        reversed.reverse();
        let backwards = warn_credential_issues(&reversed.map(at));

        assert_eq!(forwards.len(), 1);
        assert_eq!(
            forwards, backwards,
            "the same broken credential must produce the same finding regardless of scan order"
        );
    }

    /// A credential that was refused is *not* deduped: the device answered, and what it said is
    /// specific to that address. Collapsing those would re-merge exactly what the per-occurrence
    /// records exist to keep apart.
    #[test]
    fn a_refused_credential_is_still_reported_per_address() {
        let warnings = warn_credential_issues(&[
            attempted("10.0.0.1", AttemptOutcome::Rejected),
            attempted("10.0.0.2", AttemptOutcome::Rejected),
        ]);

        assert_eq!(warnings.len(), 2);
    }

    /// GH #668. Whether retrying is worth the operator's time is the one thing these have to get
    /// across, and it differs by cause: a chassis column that stopped early can recover, while
    /// records the device served malformed will come back identical. One code per cause is what
    /// keeps a single warning from having to pick one answer and be wrong for the rest.
    #[test]
    fn each_malformed_neighbour_cause_gets_its_own_code() {
        let record = |reason| MalformedNeighbours {
            ip: ip("192.168.7.244"),
            group: SnmpWalkGroup::Lldp,
            discarded: 14,
            kept: 0,
            reason,
        };

        let mut seen = Vec::new();
        for reason in [
            MalformedNeighbourReason::WalkCutShort,
            MalformedNeighbourReason::GhostRows,
            MalformedNeighbourReason::IncompleteRecords,
            MalformedNeighbourReason::UnexpectedType,
            MalformedNeighbourReason::UnreadableIndex,
        ] {
            let warnings = warn_malformed_neighbours(&[record(reason)]);
            assert_eq!(warnings.len(), 1, "{reason:?} produced {warnings:?}");
            seen.push(warnings[0].code());
        }

        let unique: std::collections::BTreeSet<_> = seen.iter().collect();
        assert_eq!(
            unique.len(),
            seen.len(),
            "two causes share a code, so one of them gets the other's advice: {seen:?}"
        );
    }

    /// Losing some neighbours and losing all of them put the device in different places on the
    /// map, so the consequence travels with the warning rather than being inferred from a count
    /// the reader does not see.
    #[test]
    fn a_partial_loss_does_not_claim_the_device_has_no_links() {
        let partial = warn_malformed_neighbours(&[MalformedNeighbours {
            ip: ip("10.0.0.1"),
            group: SnmpWalkGroup::Lldp,
            discarded: 2,
            kept: 5,
            reason: MalformedNeighbourReason::GhostRows,
        }]);
        let total = warn_malformed_neighbours(&[MalformedNeighbours {
            ip: ip("10.0.0.1"),
            group: SnmpWalkGroup::Lldp,
            discarded: 2,
            kept: 0,
            reason: MalformedNeighbourReason::GhostRows,
        }]);

        let consequence = |w: &[DiscoveryWarning]| match &w[0] {
            DiscoveryWarning::MalformedNeighboursGhostRows(detail) => detail.consequence,
            other => panic!("unexpected warning {other:?}"),
        };
        assert_eq!(
            consequence(&partial),
            MalformedNeighbourConsequence::SomeLinksLost
        );
        assert_eq!(
            consequence(&total),
            MalformedNeighbourConsequence::AllLinksLost
        );
    }

    #[test]
    fn discarding_nothing_is_not_reported() {
        assert!(
            warn_malformed_neighbours(&[MalformedNeighbours {
                ip: ip("10.0.0.1"),
                group: SnmpWalkGroup::Lldp,
                discarded: 0,
                kept: 5,
                reason: MalformedNeighbourReason::GhostRows,
            }])
            .is_empty()
        );
    }

    /// "Interfaces are missing" and "some fields are blank" are different findings, and reporting
    /// the second as possible data loss sends people hunting for interfaces that were never
    /// absent. The count each device read travels too — it used to be recorded and never shown.
    #[test]
    fn interface_walks_split_by_what_actually_fell_short() {
        let warnings = warn_incomplete_interface_walks(&[
            IncompleteInterfaceWalk {
                ip: ip("192.168.7.233"),
                collected: 3,
                set_complete: true,
            },
            IncompleteInterfaceWalk {
                ip: ip("192.168.7.242"),
                collected: 17,
                set_complete: false,
            },
        ]);

        assert_eq!(
            warnings,
            vec![
                DiscoveryWarning::InterfaceDetailsCutShort {
                    address: ip("192.168.7.233"),
                    collected: 3,
                },
                DiscoveryWarning::InterfaceSetCutShort {
                    address: ip("192.168.7.242"),
                    collected: 17,
                },
            ]
        );
    }

    fn attempted(address: &str, outcome: AttemptOutcome) -> CredentialIssue {
        CredentialIssue {
            integration: CredentialQueryPayloadDiscriminants::Snmp,
            ip: ip(address),
            reason: CredentialIssueReason::Attempted {
                outcome,
                message: "diagnostic".to_string(),
            },
            credential_id: None,
        }
    }

    /// `ATTEMPT_ORDER`'s length is compiler-checked against the variant count, but an entry that
    /// is present and unreachable would still produce nothing. Pair the compile-time check with a
    /// behavioural one, so both halves of "every outcome the product names gets said out loud"
    /// hold.
    #[test]
    fn every_reportable_outcome_produces_a_warning() {
        for outcome in AttemptOutcome::iter() {
            let warnings = warn_credential_issues(&[attempted("10.0.0.5", outcome)]);
            match outcome {
                // The user stopped the scan; nothing about that is a finding.
                AttemptOutcome::Cancelled => assert!(
                    warnings.is_empty(),
                    "{outcome:?} is not a finding but produced {warnings:?}"
                ),
                _ => assert_eq!(warnings.len(), 1, "{outcome:?} produced {warnings:?}"),
            }
        }
    }

    /// Every outcome has to reach a code of its own, or an operator hits a problem the product has
    /// a name for and is told about a different one.
    #[test]
    fn no_two_attempt_outcomes_share_a_code() {
        let mut seen = std::collections::BTreeSet::new();
        for outcome in AttemptOutcome::iter() {
            for warning in warn_credential_issues(&[attempted("10.0.0.5", outcome)]) {
                assert!(
                    seen.insert(warning.code()),
                    "{outcome:?} reuses {:?}",
                    warning.code()
                );
            }
        }
    }

    /// GH #650. A container scan that authenticated, enumerated 77 containers and then ran out of
    /// time was reported with the `TimedOut` wording — "could not be reached at that address,
    /// check the service is listening" — and on a sweep was suppressed entirely, because
    /// `already_covered_by_address_line` treats `TimedOut` as saying the same thing as an
    /// address-level warning. Neither may happen to a collection that ran out of time.
    #[test]
    fn a_collection_that_ran_out_of_time_is_not_reported_as_an_unreachable_host() {
        let address = ip("10.1.1.99");
        let warnings = warn_credential_issues(&[
            CredentialIssue {
                integration: CredentialQueryPayloadDiscriminants::PodmanSocket,
                ip: address,
                reason: CredentialIssueReason::Attempted {
                    outcome: AttemptOutcome::CollectionTimedOut,
                    message: "Integration timed out after 300s".to_string(),
                },
                credential_id: None,
            },
            // An address-level warning for the same address, which is what suppresses `TimedOut`.
            CredentialIssue {
                integration: CredentialQueryPayloadDiscriminants::Snmp,
                ip: address,
                reason: CredentialIssueReason::TargetNotResponding,
                credential_id: None,
            },
        ]);

        assert!(
            codes(&warnings).contains(&DiscoveryWarningCode::CredentialCollectionTimedOut),
            "a collection that hit its time limit must still be reported: {warnings:?}"
        );
        assert!(
            !codes(&warnings).contains(&DiscoveryWarningCode::CredentialUnreachable),
            "the service answered fine; sending the operator to check it is listening is the \
             misdiagnosis this outcome exists to prevent: {warnings:?}"
        );
    }

    /// Suppressed only when the address-level warning is actually there. Repeating "nothing
    /// answered at 10.0.0.1" per credential is the same fact twice — but deciding it by outcome
    /// alone silently swallowed every unreachable-credential report from the daemon-host phase,
    /// where no address-level warning exists because 127.0.0.1 is always up.
    #[test]
    fn an_unreachable_attempt_is_suppressed_only_when_an_address_warning_covers_it() {
        let covered = warn_credential_issues(&[
            attempted("10.0.0.1", AttemptOutcome::Unreachable),
            CredentialIssue {
                integration: CredentialQueryPayloadDiscriminants::Snmp,
                ip: ip("10.0.0.1"),
                reason: CredentialIssueReason::TargetNotResponding,
                credential_id: None,
            },
        ]);
        assert_eq!(
            codes(&covered),
            vec![DiscoveryWarningCode::CredentialTargetNotResponding],
            "the address-level warning already says this: {covered:?}"
        );

        let uncovered =
            warn_credential_issues(&[attempted("127.0.0.1", AttemptOutcome::Unreachable)]);
        assert_eq!(
            codes(&uncovered),
            vec![DiscoveryWarningCode::CredentialUnreachable],
            "no address-level warning exists for the daemon host: {uncovered:?}"
        );
    }

    /// Each pre-attempt reason names its own fix, so each is its own code: a target on no scanned
    /// subnet is a discovery-scope problem, and a closed gate is a port problem.
    #[test]
    fn each_credential_reason_gets_its_own_code() {
        // Distinct ids, so a warning built from the wrong issue would not go unnoticed. The id is
        // what lets the report open the offending credential instead of the credential list.
        let unscanned_id = Uuid::new_v4();
        let gated_id = Uuid::new_v4();
        let warnings = warn_credential_issues(&[
            CredentialIssue {
                integration: CredentialQueryPayloadDiscriminants::UnifiController,
                ip: ip("10.9.0.1"),
                reason: CredentialIssueReason::TargetNotScanned,
                credential_id: Some(unscanned_id),
            },
            CredentialIssue {
                integration: CredentialQueryPayloadDiscriminants::UnifiController,
                ip: ip("10.0.0.7"),
                reason: CredentialIssueReason::GateClosed {
                    ports: vec![PortType::new_tcp(443)],
                },
                credential_id: Some(gated_id),
            },
        ]);

        assert_eq!(
            warnings,
            vec![
                DiscoveryWarning::CredentialTargetNotScanned {
                    address: ip("10.9.0.1"),
                    integration: CredentialQueryPayloadDiscriminants::UnifiController,
                    credential_id: Some(unscanned_id),
                },
                DiscoveryWarning::CredentialGateClosed {
                    address: ip("10.0.0.7"),
                    integration: CredentialQueryPayloadDiscriminants::UnifiController,
                    ports: vec![443],
                    credential_id: Some(gated_id),
                },
            ]
        );
    }

    /// The diagnostic now belongs to the address it came from. Batching meant one message stood
    /// for a whole outcome, so a host failing for its own reason was shown someone else's.
    #[test]
    fn every_failing_address_keeps_its_own_diagnostic() {
        let issue = |address: &str, message: &str| CredentialIssue {
            integration: CredentialQueryPayloadDiscriminants::Snmp,
            ip: ip(address),
            reason: CredentialIssueReason::Attempted {
                outcome: AttemptOutcome::Rejected,
                message: message.to_string(),
            },
            credential_id: None,
        };

        let warnings = warn_credential_issues(&[
            issue("10.0.0.1", "wrong community"),
            issue("10.0.0.2", "authentication failure"),
        ]);

        let details: Vec<Option<String>> = warnings
            .iter()
            .map(|w| match w {
                DiscoveryWarning::CredentialRejected(attempt) => attempt.detail.clone(),
                other => panic!("unexpected warning {other:?}"),
            })
            .collect();
        assert_eq!(
            details,
            vec![
                Some("wrong community".to_string()),
                Some("authentication failure".to_string())
            ]
        );
    }

    /// The order two integrations answered in is what the warning is for: it says whose row
    /// stands on a port both describe. Swapping `first` and `second` on the way to the wire would
    /// send someone chasing a missing link to the wrong integration.
    #[test]
    fn equal_reach_integrations_keep_the_order_they_answered_in() {
        let warnings = warn_equal_reach_integrations(&[EqualReachIntegrations {
            ip: ip("192.168.7.250"),
            first: CredentialQueryPayloadDiscriminants::Snmp,
            second: CredentialQueryPayloadDiscriminants::Gnmi,
        }]);

        assert_eq!(
            warnings,
            vec![DiscoveryWarning::EqualReachIntegrationsMerged {
                address: ip("192.168.7.250"),
                first: CredentialQueryPayloadDiscriminants::Snmp,
                second: CredentialQueryPayloadDiscriminants::Gnmi,
            }]
        );
    }

    #[test]
    fn no_records_produces_no_warnings() {
        assert!(warn_equal_reach_integrations(&[]).is_empty());
        assert!(warn_incomplete_snmp_walks(&[]).is_empty());
        assert!(warn_incomplete_interface_walks(&[]).is_empty());
        assert!(warn_credential_issues(&[]).is_empty());
        assert!(warn_contradicted_claims(&[]).is_empty());
        assert!(warn_malformed_neighbours(&[]).is_empty());
        assert!(warn_unresolved_lldp_ports(&[]).is_empty());
    }
}
