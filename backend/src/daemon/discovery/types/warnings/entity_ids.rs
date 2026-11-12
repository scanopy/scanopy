//! The entity ids a warning carries, split out of its JSON when it is stored.

use uuid::Uuid;

use super::DiscoveryWarning;

/// The entity ids a warning carries, one mutable slot per type, `None` where the variant carries
/// no id of that type. A stored warning keeps these in foreign-key columns rather than in its
/// JSON, so the split and the rejoin both go through here.
pub struct EntityIdSlots<'a> {
    pub credential_id: Option<&'a mut Option<Uuid>>,
    pub host_id: Option<&'a mut Option<Uuid>>,
    pub remote_host_id: Option<&'a mut Option<Uuid>>,
    pub subnet_id: Option<&'a mut Option<Uuid>>,
}

impl DiscoveryWarning {
    pub fn entity_id_slots(&mut self) -> EntityIdSlots<'_> {
        let mut slots = EntityIdSlots {
            credential_id: None,
            host_id: None,
            remote_host_id: None,
            subnet_id: None,
        };
        match self {
            Self::CredentialTargetNotScanned { credential_id, .. }
            | Self::CredentialTargetNotResponding { credential_id, .. }
            | Self::CredentialGateClosed { credential_id, .. }
            | Self::CredentialDaemonOsMismatch { credential_id, .. }
            | Self::CredentialNeedsNewerDaemon { credential_id, .. }
            | Self::CredentialLoginUnavailable { credential_id, .. }
            | Self::CredentialWithoutTarget { credential_id, .. } => {
                slots.credential_id = Some(credential_id);
            }

            Self::CredentialRejected(a)
            | Self::CredentialMalformed(a)
            | Self::CredentialTlsFailed(a)
            | Self::CredentialNotThisService(a)
            | Self::CredentialCollectionFailed(a)
            | Self::CredentialCollectionTimedOut(a)
            | Self::CredentialUnreachable(a)
            | Self::CredentialTimedOut(a) => slots.credential_id = Some(&mut a.credential_id),

            Self::IntegrationWithoutCredential { host_id, .. } => slots.host_id = Some(host_id),

            Self::LldpNeighbourNotFound(n) | Self::LldpNeighbourAmbiguous(n) => {
                slots.host_id = Some(&mut n.host_id);
            }

            Self::LldpPortNoStrategy(p)
            | Self::LldpPortNotFound(p)
            | Self::LldpPortAmbiguous(p) => {
                slots.host_id = Some(&mut p.host_id);
                slots.remote_host_id = Some(&mut p.remote_host_id);
            }

            // `seen_by_host_ids` stays in the JSON: a list of ids cannot be a foreign key.
            Self::ProvisionalSubnetInferred(p) => slots.subnet_id = Some(&mut p.subnet_id),
            Self::UnconfirmedHostInferred(p) => slots.host_id = Some(&mut p.host_id),

            Self::ArpSweepTruncated { subnet_id, .. }
            | Self::SubnetProbedWithoutInterface { subnet_id, .. }
            | Self::ContainerSubnetScanned { subnet_id, .. } => slots.subnet_id = Some(subnet_id),

            Self::SnmpWalkBridgeMibAbsent { credential_id, .. }
            | Self::SnmpWalkFailed { credential_id, .. }
            | Self::SnmpCollectedNothing { credential_id, .. }
            | Self::InterfaceSetCutShort { credential_id, .. }
            | Self::InterfaceDetailsCutShort { credential_id, .. }
            | Self::SnmpWalkEntryCap { credential_id, .. }
            | Self::SnmpWalkUnsupported { credential_id, .. }
            | Self::SnmpWalkDesynchronised { credential_id, .. }
            | Self::SnmpWalkPartialDiscarded { credential_id, .. }
            | Self::SnmpWalkPartialRecorded { credential_id, .. }
            | Self::SnmpWalkNoAnswer { credential_id, .. }
            | Self::ClaimedCountReadCutShort { credential_id, .. }
            | Self::ClaimedCountUnderRead { credential_id, .. }
            | Self::ClaimedCapabilityReadCutShort { credential_id, .. }
            | Self::ClaimedCapabilityEmpty { credential_id, .. }
            | Self::LldpLocalPortDropped { credential_id, .. }
            | Self::LldpLocalPortDroppedReadCutShort { credential_id, .. }
            | Self::LldpLocalPortMisplaced { credential_id, .. } => {
                slots.credential_id = Some(credential_id);
            }

            Self::MalformedNeighboursWalkCutShort(m)
            | Self::MalformedNeighboursGhostRows(m)
            | Self::MalformedNeighboursIncompleteRecords(m)
            | Self::MalformedNeighboursUnexpectedType(m)
            | Self::MalformedNeighboursUnreadableIndex(m) => {
                slots.credential_id = Some(&mut m.credential_id);
            }

            Self::VlanRecordingFailed { .. }
            | Self::EqualReachIntegrationsMerged { .. }
            | Self::GuestOutsideScan { .. }
            | Self::GuestBehindHypervisor { .. }
            | Self::GuestIsolated { .. }
            | Self::ConnectionsWithoutProtocolResponse { .. }
            | Self::ScanTimeLimitWithEstimate { .. }
            | Self::ScanTimeLimit { .. }
            | Self::DcpSweepTimedOut { .. }
            | Self::IcmpSweepTimedOut { .. }
            | Self::ReverseDnsTimedOut { .. }
            | Self::IcmpSweepTruncated { .. }
            | Self::DeepScanConcurrencyReduced { .. }
            | Self::PortScanDescriptorsExhausted { .. }
            | Self::InterfaceFilterMatchedNothing { .. }
            | Self::ArpSweepUnavailable
            | Self::IcmpSweepUnavailable
            | Self::NeighbourResolutionIncomplete { .. }
            | Self::FdbResolutionIncomplete { .. }
            | Self::OutdatedDaemonFormat { .. }
            | Self::DaemonInDesktopVm { .. }
            | Self::WarningsTruncated { .. }
            | Self::Unknown { .. } => {}
        }
        slots
    }
}
