//! SNMP discovery integration.
//!
//! Probe: credentialed SNMP check on UDP ports 161/1161.
//! Execute: walks ifTable, queries LLDP/CDP/ARP/Entity-MIB/Bridge-FDB,
//!          enriches HostData with system info, ip_addresses, and interfaces.
//!
//! Also contains low-level SNMP utilities (queries, session management, OIDs, types).

pub mod oids;
pub mod queries;
pub mod session;
pub mod types;
pub mod values;

mod convert;
mod local_ports;

/// The simulated devices behind `tools/snmp/`. Test and generator only — see `sim::wire`.
#[cfg(any(test, feature = "snmp-sim"))]
pub mod sim;

// Re-export commonly used items
pub use queries::{IfTableWalk, SnmpCollection};
pub use queries::{
    query_arp_table, query_bridge_fdb, query_bridge_port_mapping, query_cdp_neighbors,
    query_entity_physical, query_ip_addr_table, query_lldp_local, query_lldp_local_ports,
    query_lldp_neighbors, query_port_vlan_membership, query_system_info, query_vlan_table,
    walk_if_table,
};
pub use session::SNMP_WALK_TIMEOUT;
use session::{SNMP_PROBE_TIMEOUT, SnmpContext, bridge_context, create_session};
pub use types::{
    ArpEntry, BridgeFdbEntry, CdpNeighbor, DeviceInventory, IfTableEntry, IpAddrEntry,
    LldpLocalInfo, LldpLocalPort, LldpNeighbor, PortVlanMembership, SystemInfo, VlanInfo,
};

pub(crate) use convert::convert_snmp_if_entry;
pub use local_ports::LocalPortOutcome;
pub(crate) use local_ports::{count_dropped_neighbours, remap_lldp_local_ports};
// Reached through `snmp::` only by the simulator's device tests.
#[cfg(test)]
pub(crate) use local_ports::unique_interface_macs;

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use mac_address::MacAddress;
use tokio::time::timeout;
use tracing::debug;
use uuid::Uuid;

use crate::{
    daemon::utils::scanner::{SnmpProbeOutcome, try_snmp_with_credential_on_port},
    server::{
        credentials::r#impl::{
            mapping::{
                CredentialQueryPayload, CredentialQueryPayloadDiscriminants, SnmpQueryCredential,
            },
            types::CredentialAssignment,
        },
        hosts::r#impl::base::{Host, HostBase},
        interface_neighbors::r#impl::base::InterfaceNeighborEvidence,
        interfaces::r#impl::base::{
            IfAdminStatus, IfOperStatus, Interface, InterfaceBase, InterfaceDataComplete, if_type,
        },
        ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
        ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue},
        lldp::{LldpChassisId, LldpPortId},
        ports::r#impl::base::PortType,
        services::r#impl::patterns::ClientProbe,
        shared::attribution::AttributeSource,
        shared::types::entities::EntitySource,
        subnets::r#impl::base::Subnet,
    },
};

use super::{
    Checkpoint, Completeness, DiscoveryIntegration, IntegrationContext, IntegrationFailure,
    InterfaceViewScope, ProbeContext, ProbeFailure, ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;
pub use crate::daemon::discovery::service::warnings::LocalPortPlacementReason;
use crate::daemon::discovery::service::warnings::{
    AttemptOutcome, ClaimSource, DeviceClaim, IncompleteInterfaceWalk, MalformedNeighbours,
    SnmpCollectedNothing, SnmpCollectionOutcome, SnmpGroupOutcome, SnmpWalkGroup,
    UnresolvedLldpPorts, contradicted_claims, snmp_walk_shortfalls,
};

/// Handle returned by a successful SNMP probe — carries the working credential and port.
pub struct SnmpProbeHandle {
    pub credential: SnmpQueryCredential,
    pub port: u16,
}

/// Run one SNMP query under `SNMP_WALK_TIMEOUT`, collapsing both a query error and a
/// timeout into `T::default()` — the empty/`None` fallback every call site already used
/// for errors alone.
///
/// Without this, a single query that never returns consumes the whole
/// `SnmpIntegration::timeout()` budget and the integration is aborted mid-sequence,
/// discarding everything collected so far. Observed on Ubiquiti switches, where
/// `query_bridge_fdb` hangs and the host ends up created with zero interfaces.
async fn query_or_default<T, Fut>(ip: IpAddr, query: &str, fut: Fut) -> T
where
    T: Default,
    Fut: std::future::Future<Output = Result<T>>,
{
    match timeout(SNMP_WALK_TIMEOUT, fut).await {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            debug!(ip = %ip, query, error = %e, "SNMP query failed");
            T::default()
        }
        Err(_) => {
            debug!(ip = %ip, query, "SNMP query timed out");
            T::default()
        }
    }
}

/// How much a probe outcome tells us, for picking between the two SNMP ports' answers.
///
/// A device that refuses us on 161 and ignores us on 1161 has told us something on 161; reporting
/// the silence would be reporting the less informative of the two.
fn probe_specificity(outcome: AttemptOutcome) -> u8 {
    match outcome {
        AttemptOutcome::Rejected => 3,
        AttemptOutcome::NotThisService | AttemptOutcome::Malformed => 2,
        _ => 1,
    }
}

pub struct SnmpIntegration;

#[async_trait]
impl DiscoveryIntegration for SnmpIntegration {
    /// An ifTable walk is the device's own account of every interface it has.
    fn interface_view_scope(&self) -> InterfaceViewScope {
        InterfaceViewScope::FullIfTable
    }

    fn credential_type(&self) -> CredentialQueryPayloadDiscriminants {
        CredentialQueryPayloadDiscriminants::Snmp
    }

    fn estimated_seconds(&self) -> u32 {
        15
    }

    /// Must exceed the sum of every sequential walk's own timeout, or the outer cap silently
    /// kills the walks that run last — bridge FDB and per-port VLAN membership — which is
    /// exactly the data operators were reporting as missing. 13 walks at
    /// [`session::SNMP_WALK_TIMEOUT`] each is the worst case; this leaves headroom above it.
    fn timeout(&self) -> Duration {
        Duration::from_secs(900)
    }

    // No probe_gate_ports — SNMP does its own UDP port probing.

    async fn probe(&self, ctx: &ProbeContext<'_>) -> Result<ProbeSuccess, ProbeFailure> {
        let snmp_cred = match ctx.credential {
            CredentialQueryPayload::Snmp(cred) => cred,
            _ => return Err(ProbeFailure::malformed("Expected SNMP credential")),
        };

        let snmp_ports: &[u16] = &[161, 1161];

        // The most specific answer any port gave. A device listening on 161 and silent on 1161
        // should be reported as whatever 161 said, not as the silence from 1161 — so a refusal
        // outranks a timeout, which outranks nothing having been tried.
        let mut best: Option<(AttemptOutcome, String)> = None;

        for &port in snmp_ports {
            if ctx.cancel.is_cancelled() {
                return Err(ProbeFailure::cancelled());
            }

            // Cap the whole probe (create-session + GET) so a non-responder — v3's
            // engine-discovery especially — costs ~2s instead of up to 7s.
            let port_outcome = match timeout(
                SNMP_PROBE_TIMEOUT,
                try_snmp_with_credential_on_port(ctx.ip, snmp_cred, port),
            )
            .await
            {
                Ok(outcome) => outcome,
                Err(_) => SnmpProbeOutcome::Failed(
                    AttemptOutcome::TimedOut,
                    format!("no answer on port {port} within {SNMP_PROBE_TIMEOUT:?}"),
                ),
            };

            match port_outcome {
                SnmpProbeOutcome::Answered(detected_port) => {
                    return Ok(ProbeSuccess {
                        client_probe: ClientProbe::Snmp,
                        ports: vec![PortType::new_udp(detected_port)],
                        handle: Some(Box::new(SnmpProbeHandle {
                            credential: snmp_cred.clone(),
                            port: detected_port,
                        })),
                    });
                }
                SnmpProbeOutcome::Failed(outcome, message) => {
                    tracing::debug!(
                        ip = %ctx.ip,
                        port,
                        ?outcome,
                        error = %message,
                        "SNMP credential probe failed"
                    );
                    if best.as_ref().is_none_or(|(seen, _)| {
                        probe_specificity(outcome) > probe_specificity(*seen)
                    }) {
                        best = Some((outcome, format!("port {port}: {message}")));
                    }
                }
            }
        }

        // No "public" fallback here — the daemon injects a broadcast SNMP credential
        // with community "public" into credential_mappings, so it's tried as its own
        // integration dispatch. No special-casing needed.

        let (outcome, message) = best.unwrap_or((
            AttemptOutcome::TimedOut,
            format!("SNMP not responding on {}", ctx.ip),
        ));
        Err(ProbeFailure::with_outcome(outcome, message))
    }

    async fn execute(
        &self,
        ctx: &IntegrationContext<'_>,
        host_data: &mut HostData,
        checkpoint: &Checkpoint<'_>,
    ) -> Result<Completeness, IntegrationFailure> {
        // Downcast probe handle to get the working credential and port
        let handle = ctx
            .probe_handle
            .and_then(|h| h.downcast_ref::<SnmpProbeHandle>())
            .ok_or_else(|| anyhow::anyhow!("SNMP execute called without SnmpProbeHandle"))?;

        let credential = &handle.credential;
        let port = handle.port;
        let ip = ctx.ip;

        // Open one SNMP session per host and reuse it across every query below.
        // Previously each of the ~12 queries opened its own session — and for v3 each
        // repeated the full engine-discovery handshake — so a single collection did
        // ~12 session setups. Reusing one session removes that per-query cost.
        let mut session = match create_session(ip, credential, port, SnmpContext::Default).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    ip = %ip,
                    error = %e,
                    "Failed to open SNMP session; skipping SNMP collection"
                );
                return Ok(Completeness::Complete);
            }
        };

        // Query system info
        let info = query_or_default(ip, "system_info", query_system_info(&mut session, ip)).await;
        let system_info = if info.sys_descr.is_some()
            || info.sys_name.is_some()
            || info.sys_object_id.is_some()
        {
            tracing::debug!(
                ip = %ip,
                sys_name = ?info.sys_name,
                "SNMP system info retrieved"
            );
            Some(info)
        } else {
            tracing::debug!(ip = %ip, "SNMP system_info returned no data");
            None
        };

        if ctx.cancel.is_cancelled() {
            return Err(IntegrationFailure::cancelled());
        }

        // Walk interface table. `if_table_complete` tells the server whether this is an
        // authoritative full ifTable (safe to prune stale interfaces against) or a partial walk
        // cut short by timeout/error (must NOT prune — see GH #649). A hard failure yields an
        // empty set, which the server's existing empty-set guard already protects.
        let if_table = query_or_default(ip, "if_table", walk_if_table(&mut session, ip)).await;
        let snmp_if_entries = if_table.entries;
        tracing::debug!(
            ip = %ip,
            if_count = snmp_if_entries.len(),
            set_complete = if_table.set_complete,
            attributes_complete = if_table.attributes_complete,
            "SNMP ifTable walked"
        );

        // Persist the interface set before the slower enrichment queries below, so a hang in any
        // later query cannot strand the host with zero interfaces. This is the one deliberate
        // mid-flight commit in the codebase; everything else is atomic.
        //
        // `InterfaceDataComplete::none()` rather than the default is load-bearing. None of the
        // neighbour/FDB/VLAN walks has run at this point, so no group is authoritative, and
        // claiming otherwise makes the server clear the very columns this checkpoint exists to
        // protect. Pruning acts on the interface *set*, so `set_complete` is what gates it — not
        // whether every attribute column also finished (#649).
        let network_id = host_data.host.base.network_id;
        let no_vlan_uuids = std::collections::HashMap::new();
        host_data.contribute_interfaces(
            ctx.interface_source,
            snmp_if_entries
                .iter()
                .map(|entry| {
                    // ipAddrTable hasn't been queried yet at this early checkpoint (see below) —
                    // every row reports `ip_configured: false` here and gets the real value once the
                    // authoritative interface set is written later in this same poll.
                    convert_snmp_if_entry(
                        entry,
                        network_id,
                        &[],
                        &[],
                        &[],
                        &[],
                        &no_vlan_uuids,
                        &HashSet::new(),
                    )
                })
                .collect(),
            if_table.set_complete,
            InterfaceDataComplete::none(),
        );
        checkpoint.commit(host_data);

        // Record an incomplete walk on the session rather than leaving it to debug logs, keeping
        // which kind it was: a short interface list means interfaces are genuinely missing, while
        // a short attribute column only means some fields are blank. Reporting the second as
        // possible data loss sends operators hunting for interfaces that were never absent.
        // Rendered to one line per run at finalize — one paragraph per device drowns the
        // notification on any real network.
        // What the device said to expect, kept beside what was read so the two can be compared
        // once every walk has run. Read from the system group, which is why they are held here
        // rather than derived at the comparison site.
        //
        // `ifNumber` is only a claim worth checking when the device published a positive figure:
        // agents that do not implement it answer nothing, and one reporting zero interfaces while
        // serving none has not contradicted itself.
        let if_number_claim = system_info
            .as_ref()
            .and_then(|info| info.if_number)
            .filter(|count| *count > 0)
            .map(|count| DeviceClaim::Count {
                source: ClaimSource::IfNumber,
                expected: count as usize,
            });
        // Bit 2 of sysServices is the datalink layer: a device that sets it says it bridges.
        // Weaker than `dot1dBaseNumPorts` because it carries no count, so it can only ever catch
        // a bridge table that came back completely empty.
        let bridge_bit_claim = system_info
            .as_ref()
            .and_then(|info| info.sys_services)
            .is_some_and(|services| services & 0x02 != 0)
            .then_some(DeviceClaim::Implements {
                source: ClaimSource::SysServicesBridgeBit,
            });
        let if_set_complete = if_table.set_complete;

        let walk_fell_short = !if_table.set_complete || !if_table.attributes_complete;
        if !snmp_if_entries.is_empty() && walk_fell_short {
            ctx.ops
                .record_interface_shortfall(IncompleteInterfaceWalk {
                    ip,
                    collected: snmp_if_entries.len(),
                    set_complete: if_table.set_complete,
                })
                .await;
        }

        // Query LLDP neighbors
        let lldp = query_or_default(ip, "lldp", query_lldp_neighbors(&mut session, ip)).await;
        // Two different questions, deliberately not one flag.
        //
        // `lldp_complete` — did the walk finish? An agent with no LLDP-MIB answers immediately
        // and completely, so this stays true and no shortfall is reported. Warning about it
        // every scan would be the same noise the bridge-MIB groups used to produce.
        //
        // `lldp_authoritative` — may this result overwrite what the server holds? Only a device
        // that *has* the MIB and reports no neighbours is saying "there are none". Answering
        // `noSuchObject` says nothing about neighbours, and treating it as authority erased the
        // rows the UniFi integration writes for these very switches — the only source of LLDP
        // they have — whenever the SNMP pass happened to land second.
        let lldp_complete = lldp.complete;
        let lldp_reason = lldp.reason;
        let lldp_authoritative = lldp.complete && !lldp.unsupported;
        let lldp_discarded = lldp.discarded;
        let lldp_discard_reason = lldp.discard_reason;
        let lldp_local_port_is_if_index = lldp.local_port_is_if_index;
        let mut lldp_neighbors = lldp.records;
        tracing::debug!(
            ip = %ip,
            count = lldp_neighbors.len(),
            complete = lldp_complete,
            unsupported = lldp.unsupported,
            "LLDP neighbors discovered"
        );
        let lldp_count = lldp_neighbors.len();

        // Query CDP neighbors (Cisco devices)
        let cdp = query_or_default(ip, "cdp", query_cdp_neighbors(&mut session, ip)).await;
        let cdp_complete = cdp.complete;
        let cdp_reason = cdp.reason;
        let cdp_discarded = cdp.discarded;
        let cdp_discard_reason = cdp.discard_reason;
        let cdp_neighbors = cdp.records;
        tracing::debug!(
            ip = %ip,
            count = cdp_neighbors.len(),
            complete = cdp_complete,
            "CDP neighbors discovered"
        );
        let cdp_count = cdp_neighbors.len();

        // Records the device served and we could not use. Reported per group because the
        // consequence differs — losing every neighbour on a switch takes it off L2 Physical
        // entirely, losing some leaves it there with holes — and because no rescan will change
        // either, which is the part an operator most needs told (GH #668).
        for (group, discarded, kept, reason) in [
            (
                SnmpWalkGroup::Lldp,
                lldp_discarded,
                lldp_count,
                lldp_discard_reason,
            ),
            (
                SnmpWalkGroup::Cdp,
                cdp_discarded,
                cdp_count,
                cdp_discard_reason,
            ),
        ] {
            // No reason means nothing was thrown away, and there is nothing to report.
            if let Some(reason) = reason {
                ctx.ops
                    .record_malformed_neighbours(MalformedNeighbours {
                        ip,
                        group,
                        discarded,
                        kept,
                        reason,
                    })
                    .await;
            }
        }

        // Translate LLDP local-port indices (which are lldpLocPortNum values, a
        // separate namespace from ifIndex on vendors like ExtremeXOS) to real ifIndex
        // values so neighbours attach to the correct interface. Resolved via
        // lldpLocPortTable; falls back to identity (correct for VOSS and any device
        // that reports lldpLocPortNum == ifIndex or omits the table). CDP is not
        // remapped: cdpCacheIfIndex is already a real ifIndex.
        // Not walked at all when there is no neighbour to place, so its outcome is "nothing to
        // ask" rather than a complete read of an empty table. Nor when the neighbours came from
        // the LLDP-V2-MIB (GH #688): `lldpV2RemLocalIfIndex` is already an ifIndex, and the
        // classic `lldpLocPortTable` is not a table such a device serves.
        let lldp_local_ports = if lldp_count > 0 && !lldp_local_port_is_if_index {
            query_or_default(
                ip,
                "lldp_local_ports",
                query_lldp_local_ports(&mut session, ip),
            )
            .await
        } else {
            SnmpCollection::skipped()
        };
        let lldp_local_ports_outcome = SnmpGroupOutcome {
            complete: lldp_local_ports.complete,
            observed: lldp_local_ports.records.len(),
            reason: lldp_local_ports.reason,
            claim: lldp_local_ports.claim,
        };
        let lldp_local_ports = lldp_local_ports.records;
        let local_ports = if lldp_local_port_is_if_index {
            // Nothing to translate, but the placement rule still applies: a neighbour on an
            // ifIndex the interface walk did not return reaches no interface either way.
            LocalPortOutcome {
                unmatched: 0,
                dropped: count_dropped_neighbours(
                    &lldp_neighbors,
                    &lldp_local_ports,
                    &snmp_if_entries,
                ),
            }
        } else {
            remap_lldp_local_ports(&mut lldp_neighbors, &lldp_local_ports, &snmp_if_entries)
        };
        if local_ports.unmatched > 0 || local_ports.dropped > 0 {
            tracing::warn!(
                ip = %ip,
                unmatched = local_ports.unmatched,
                dropped = local_ports.dropped,
                total = lldp_count,
                "LLDP neighbours could not be placed on a local interface; the dropped ones \
                 contribute no link at all"
            );
            ctx.ops
                .record_unresolved_lldp_ports(UnresolvedLldpPorts {
                    ip,
                    unresolved: local_ports.unmatched,
                    dropped: local_ports.dropped,
                    total: lldp_count,
                    // Both reads, because either one being short costs a placement — and because
                    // telling an operator their switch numbers its LLDP ports separately, on a
                    // device whose port table we only half read, is a diagnosis of a fault it
                    // does not have (GH #668).
                    reason: LocalPortPlacementReason::from_reads(
                        lldp_local_ports_outcome.complete,
                        if_set_complete,
                    ),
                })
                .await;
        }

        // Query ipAddrTable for IP->ifIndex+netMask mappings
        let ip_addr_table =
            query_or_default(ip, "ip_addr_table", query_ip_addr_table(&mut session, ip)).await;
        let ip_addresses_outcome = SnmpGroupOutcome {
            complete: ip_addr_table.complete,
            observed: ip_addr_table.records.len(),
            reason: ip_addr_table.reason,
            claim: ip_addr_table.claim,
        };
        let ip_addr_table = ip_addr_table.records;
        // ifIndexes the device's own ipAddrTable binds an IP address to. On a host that exposes
        // one MAC across a real NIC and several NDIS filter/LWF pseudo-interfaces (GH #668), only
        // the real NIC's ifIndex ever appears here — a filter driver is not a distinct entry the
        // IP stack configures an address on. Kept independent of `ip_address_id`, which requires
        // the MAC to be unique on the host before it links anything and is therefore blank for
        // exactly the hosts this signal exists to help.
        let ip_configured_if_indexes: HashSet<i32> =
            ip_addr_table.values().map(|entry| entry.if_index).collect();

        // Query ARP table for remote host discovery
        let arp = query_or_default(ip, "arp", query_arp_table(&mut session, ip)).await;
        let arp_outcome = SnmpGroupOutcome {
            complete: arp.complete,
            observed: arp.records.len(),
            reason: arp.reason,
            claim: arp.claim,
        };
        let arp_entries = arp.records;
        let arp_count = arp_entries.len();
        tracing::info!(
            ip = %ip,
            count = arp_count,
            complete = arp_outcome.complete,
            "ARP table entries collected"
        );

        // Query ENTITY-MIB for hardware inventory
        let device_inventory =
            query_or_default(ip, "entity_mib", query_entity_physical(&mut session, ip)).await;
        let device_inventory_outcome = SnmpGroupOutcome {
            complete: device_inventory.complete,
            observed: usize::from(device_inventory.records.is_some()),
            reason: device_inventory.reason,
            claim: device_inventory.claim,
        };
        let device_inventory = device_inventory.records;
        let has_entity_inventory = device_inventory.is_some();
        tracing::info!(
            ip = %ip,
            has_inventory = has_entity_inventory,
            "ENTITY-MIB inventory queried"
        );

        // The bridge and VLAN tables, and only those, come from the credential's context when it
        // names one. Cisco IOS-XE partitions its forwarding database per VLAN and keeps a
        // near-empty one in the default context, which is how a switch with a full FDB reported a
        // single entry (GH #686). Everything else — ifTable, LLDP, ARP, the system MIB — stays on
        // the default-context session: those live there on every device, and only one SNMP
        // credential per host ever executes, so moving the whole session into a bridge context
        // would take the interfaces with it.
        //
        // A second session costs a second v3 engine-discovery handshake, so it is opened only
        // when a context is actually configured. If it cannot be opened, the default-context
        // session is used and the shortfall reporting says what it found, rather than the scan
        // losing its bridge data outright.
        let mut context_session = match bridge_context(credential) {
            Some(name) => {
                match create_session(ip, credential, port, SnmpContext::FromCredential).await {
                    Ok(s) => {
                        tracing::debug!(ip = %ip, context = name, "Opened bridge-context session");
                        Some(s)
                    }
                    Err(e) => {
                        tracing::warn!(
                            ip = %ip,
                            context = name,
                            error = %e,
                            "Could not open the credential's bridge context; reading bridge and \
                             VLAN tables from the default context instead"
                        );
                        None
                    }
                }
            }
            None => None,
        };
        let bridge_session = context_session.as_mut().unwrap_or(&mut session);

        // Walk dot1dBasePortIfIndex once and share it. Both the bridge FDB and per-port VLAN
        // membership are keyed by bridge port, and each used to walk this table for itself —
        // so a switch that answers the OID with silence rather than `noSuchObject` (the
        // Ubiquiti USW-Pro-Max does) paid the walk timeout twice per scan for a table that
        // was never going to arrive.
        let bridge_ports = query_or_default(
            ip,
            "bridge_port_mapping",
            query_bridge_port_mapping(bridge_session, ip),
        )
        .await;
        tracing::debug!(
            ip = %ip,
            count = bridge_ports.records.len(),
            complete = bridge_ports.complete,
            "Bridge port mappings collected"
        );

        // Query bridge FDB for MAC-to-port mappings
        let fdb = query_or_default(
            ip,
            "bridge_fdb",
            query_bridge_fdb(bridge_session, ip, &bridge_ports),
        )
        .await;
        let fdb_complete = fdb.complete;
        let fdb_reason = fdb.reason;
        // Same distinction the LLDP walk draws, and for the same reason: a device that answers
        // `noSuchObject` for both forwarding tables has said nothing about its MACs, so an empty
        // result is not authority to clear the ones already stored. It reads as a clean, complete,
        // empty table otherwise — which is how a Catalyst queried without its per-VLAN context
        // came to overwrite a good forwarding database with almost nothing (GH #686).
        let fdb_authoritative = fdb.complete && !fdb.unsupported;
        let bridge_fdb = fdb.records;
        let fdb_count = bridge_fdb.len();
        tracing::info!(
            ip = %ip,
            count = fdb_count,
            complete = fdb_complete,
            "Bridge FDB entries collected"
        );

        // Query VLAN table for VLAN names and persist as VLAN entities
        let vlan_table =
            query_or_default(ip, "vlan_table", query_vlan_table(bridge_session, ip)).await;
        let vlan_names_outcome = SnmpGroupOutcome {
            complete: vlan_table.complete,
            observed: vlan_table.records.len(),
            reason: vlan_table.reason,
            claim: vlan_table.claim,
        };
        let vlan_table = vlan_table.records;
        let vlan_number_to_uuid: std::collections::HashMap<u16, Uuid> = if !vlan_table.is_empty() {
            tracing::info!(
                ip = %ip,
                count = vlan_table.len(),
                vlans = ?vlan_table.iter().map(|v| format!("{}={}", v.vlan_id, v.name)).collect::<Vec<_>>(),
                "VLAN table entries collected"
            );
            match ctx.ops.upsert_vlans(&vlan_table, network_id).await {
                Ok(mapping) => mapping,
                Err(e) => {
                    tracing::warn!(ip = %ip, error = %e, "Failed to upsert VLANs, VLAN IDs will not be resolved");
                    // The switch answered in full and we could not record it. Silent until now,
                    // and the consequence is not small — every interface on this device loses
                    // its VLAN ids, which looks identical to a switch that reports no VLANs.
                    ctx.ops.record_vlan_recording_failure(ip).await;
                    std::collections::HashMap::new()
                }
            }
        } else {
            std::collections::HashMap::new()
        };

        // Query per-port VLAN membership
        let port_vlan_membership = query_or_default(
            ip,
            "port_vlan_membership",
            query_port_vlan_membership(bridge_session, ip, &bridge_ports),
        )
        .await;
        let vlan_membership_complete = port_vlan_membership.complete;
        let vlan_membership_reason = port_vlan_membership.reason;
        let port_vlan_membership = port_vlan_membership.records;
        tracing::info!(
            ip = %ip,
            count = port_vlan_membership.len(),
            complete = vlan_membership_complete,
            "Port VLAN memberships collected"
        );

        // Query local LLDP identity
        let lldp_local =
            query_or_default(ip, "lldp_local", query_lldp_local(&mut session, ip)).await;
        tracing::info!(
            ip = %ip,
            has_lldp_local = lldp_local.is_some(),
            "LLDP local identity queried"
        );

        // --- MAC enrichment from ipAddrTable when ARP didn't provide one ---
        if let Some(ip_entry) = ip_addr_table.get(&ip)
            && let Some(entry) = snmp_if_entries
                .iter()
                .find(|e| e.if_index == ip_entry.if_index)
            && let Some(mac) = entry.if_phys_address
        {
            tracing::debug!(
                ip = %ip,
                if_index = ip_entry.if_index,
                mac = ?mac,
                "ipAddrTable MAC enrichment"
            );
            // The device's own ipAddrTable, matched to its own ifPhysAddress.
            host_data.with_mac_for_ip(ip, mac, AttributeSource::Probe(ClientProbe::Snmp));
        }

        // --- Enrich host fields from SNMP system info ---
        //
        // Two sources over one session. `sysDescr`, `sysObjectID` and `sysName` are what the
        // device says about itself; `sysLocation` and `sysContact` are what an operator typed
        // into it, which we are only relaying — so they carry a person's intent and outrank
        // anything a machine emitted, including the rest of this same walk.
        if let Some(ref info) = system_info {
            let probe = AttributeSource::Probe(ClientProbe::Snmp);
            let authored = AttributeSource::Authored(ClientProbe::Snmp);
            if let Some(ref v) = info.sys_descr {
                host_data.with_sys_descr(v.clone(), probe);
            }
            if let Some(ref v) = info.sys_object_id {
                host_data.with_sys_object_id(v.clone(), probe);
            }
            if let Some(ref v) = info.sys_location {
                host_data.with_sys_location(v.clone(), authored);
            }
            if let Some(ref v) = info.sys_contact {
                host_data.with_sys_contact(v.clone(), authored);
            }
            if let Some(ref v) = info.sys_name {
                host_data.with_sys_name(v.clone(), probe);
            }
        }

        // --- Set chassis_id from LLDP local identity ---
        if let Some(ref local) = lldp_local
            && let Some(chassis) =
                LldpChassisId::from_snmp(local.chassis_id_subtype, &local.chassis_id_bytes)
        {
            // Same canonical form the server matches a *neighbor's* chassis ID against, so a
            // device whose chassis MAC appears on none of its ports is still identifiable.
            host_data.with_chassis_id(
                chassis.identifier(),
                AttributeSource::Probe(ClientProbe::Snmp),
            );
        }

        // --- Add ENTITY-MIB hardware inventory ---
        if let Some(ref inventory) = device_inventory {
            let probe = AttributeSource::Probe(ClientProbe::Snmp);
            if let Some(ref v) = inventory.manufacturer {
                host_data.with_manufacturer(v.clone(), probe);
            }
            if let Some(ref v) = inventory.model {
                host_data.with_model(v.clone(), probe);
            }
            if let Some(ref v) = inventory.serial_number {
                host_data.with_serial_number(v.clone(), probe);
            }
            // Two columns, two fields. `entPhysicalFirmwareRev` and `entPhysicalSoftwareRev` are
            // distinct objects in RFC 4133 — on a Cisco chassis the bootloader and the IOS
            // version — so neither is folded into the other.
            if let Some(ref v) = inventory.firmware_revision {
                host_data.with_firmware_revision(v.clone(), probe);
            }
            if let Some(ref v) = inventory.software_revision {
                host_data.with_software_revision(v.clone(), probe);
            }
        }

        // --- Credential assignment for the working SNMP credential ---
        if let Some(cred_id) = ctx.credential_id {
            host_data.add_credential_assignment(CredentialAssignment {
                credential_id: cred_id,
                ip_address_ids: None,
            });
        }

        // --- Convert SNMP ifTable entries to Interface entities ---
        // Replaces (not appends to) the bare set persisted right after the ifTable walk, now that
        // the neighbour/FDB/VLAN queries have supplied the enrichment those bare entries lacked.
        host_data.contribute_interfaces(
            ctx.interface_source,
            snmp_if_entries
                .iter()
                .map(|entry| {
                    convert_snmp_if_entry(
                        entry,
                        network_id,
                        &lldp_neighbors,
                        &cdp_neighbors,
                        &bridge_fdb,
                        &port_vlan_membership,
                        &vlan_number_to_uuid,
                        &ip_configured_if_indexes,
                    )
                })
                .collect(),
            // Whether this is a complete, authoritative ifTable. The server only prunes
            // interfaces no longer reported when this is true, so a partial walk cannot tear
            // down the host's L2 topology (GH #649).
            if_table.set_complete,
            // Which groups the server may treat as authoritative. A group we only read partially
            // must not overwrite what is already stored — an empty result from a cut-short walk
            // is indistinguishable from a device reporting nothing, and for the neighbour fields
            // losing them drops the row out of L2 resolution for good.
            InterfaceDataComplete {
                lldp: lldp_authoritative,
                cdp: cdp_complete,
                fdb: fdb_authoritative,
                vlan_membership: vlan_membership_complete,
            },
        );

        // A cut-short neighbour walk used to be entirely silent — it took a database query to
        // discover that a switch had lost its chassis ids. Record it so the run can say so once,
        // with what happened as a result: the previous values are kept, so this is a "no fresh
        // data" notice rather than a loss.
        //
        // `returned_any` is carried per group because it separates two different problems that
        // share the `complete: false` flag: a walk that returned rows and stopped was truncated,
        // while one that returned nothing timed out or errored outright.
        //
        // Which groups are worth reporting — and which are merely downstream of a failure
        // already being reported — is `snmp_walk_shortfalls`'s call, so it can be tested
        // without a live agent.
        let collection_outcome = SnmpCollectionOutcome {
            lldp: SnmpGroupOutcome {
                complete: lldp_complete,
                observed: lldp_count,
                reason: lldp_reason,
                // A device that answered `lldpLocChassisId` runs an LLDP agent, so an empty
                // neighbour table from it is worth a second look — #685 is precisely that pair.
                claim: lldp_local.as_ref().map(|_| DeviceClaim::Implements {
                    source: ClaimSource::LldpLocalIdentity,
                }),
            },
            cdp: SnmpGroupOutcome {
                complete: cdp_complete,
                observed: cdp_count,
                reason: cdp_reason,
                claim: None,
            },
            interfaces: SnmpGroupOutcome {
                complete: if_set_complete,
                observed: snmp_if_entries.len(),
                reason: None,
                claim: if_number_claim,
            },
            bridge_port_numbering: SnmpGroupOutcome {
                complete: bridge_ports.complete,
                observed: bridge_ports.records.len(),
                reason: bridge_ports.reason,
                claim: bridge_ports.claim.or(bridge_bit_claim),
            },
            bridge_forwarding: SnmpGroupOutcome {
                complete: fdb_complete,
                observed: fdb_count,
                reason: fdb_reason,
                claim: None,
            },
            vlan_membership: SnmpGroupOutcome {
                complete: vlan_membership_complete,
                observed: port_vlan_membership.len(),
                reason: vlan_membership_reason,
                claim: None,
            },
            arp_table: arp_outcome,
            device_inventory: device_inventory_outcome,
            ip_addresses: ip_addresses_outcome,
            lldp_local_ports: lldp_local_ports_outcome,
            vlan_names: vlan_names_outcome,
        };

        let incomplete = snmp_walk_shortfalls(ip, collection_outcome);
        ctx.ops.record_snmp_shortfalls(incomplete).await;

        // Separate from the shortfalls above, and emitted alongside them rather than instead of
        // them: a shortfall says why *we* stopped reading, a contradiction says what the *device*
        // said was there. A device that misreports its own count still scans — everything read is
        // already recorded by this point, and nothing here can fail a collection.
        let contradicted = contradicted_claims(ip, collection_outcome);
        if !contradicted.is_empty() {
            ctx.ops.record_contradicted_claims(contradicted).await;
        }

        // A device that answered the credential and then produced nothing from any table.
        //
        // The per-group lines above cannot say this. Each of them reports a walk that fell
        // *short* of the others, and a device where every walk ends cleanly on an empty table
        // has no group to single out — so GH #674's switch was logged five times at INFO with
        // `count=0` and reported to the operator as a clean scan. The probe already proved the
        // address, port and community are right, which is what makes silence worth a line.
        //
        // Deliberately every group, not any: one interface or one neighbour means SNMP is
        // working and this is a device with little to say, which is not worth warning about.
        let collected_nothing = snmp_if_entries.is_empty()
            && lldp_count == 0
            && cdp_count == 0
            && arp_count == 0
            && fdb_count == 0
            && port_vlan_membership.is_empty()
            && vlan_table.is_empty()
            && ip_addr_table.is_empty()
            && device_inventory.is_none();
        if collected_nothing {
            tracing::warn!(
                ip = %ip,
                "SNMP probe succeeded but every table came back empty"
            );
            ctx.ops
                .record_snmp_collected_nothing(SnmpCollectedNothing { ip })
                .await;
        }

        // --- Discover remote subnets from ipAddrTable ---
        let scanning_subnet = ctx.scanning_subnet;
        let mut discovered_subnets: Vec<Subnet> = Vec::new();

        for (entry_ip, entry) in &ip_addr_table {
            let mask = match entry.net_mask {
                Some(m) => m,
                None => continue,
            };

            // Only handle IPv4
            let (entry_ipv4, mask_ipv4) = match (entry_ip, mask) {
                (IpAddr::V4(eip), IpAddr::V4(mip)) => (*eip, mip),
                _ => continue,
            };

            // Skip loopback, link-local
            let octets = entry_ipv4.octets();
            if octets[0] == 127 || (octets[0] == 169 && octets[1] == 254) {
                continue;
            }

            // Skip /32 and /0
            let mask_octets = mask_ipv4.octets();
            let mask_u32 = u32::from_be_bytes(mask_octets);
            if mask_u32 == 0xFFFFFFFF || mask_u32 == 0 {
                continue;
            }

            // Build network from IP + mask
            let ipv4_network = match ipnetwork::Ipv4Network::with_netmask(entry_ipv4, mask_ipv4) {
                Ok(n) => n,
                Err(_) => continue,
            };
            let ip_network = ipnetwork::IpNetwork::V4(ipv4_network);

            // Skip if this is the current scanning subnet
            if let Some(subnet) = scanning_subnet {
                let new_cidr_str = format!("{}/{}", ipv4_network.network(), ipv4_network.prefix());
                if new_cidr_str == subnet.base.cidr.to_string() {
                    continue;
                }
            }

            // Get interface name for subnet typing
            let if_name = snmp_if_entries
                .iter()
                .find(|e| e.if_index == entry.if_index)
                .and_then(|e| e.if_name.clone())
                .unwrap_or_default();

            if let Some(new_subnet) = Subnet::from_discovery(if_name, &ip_network, network_id) {
                tracing::info!(
                    ip = %ip,
                    cidr = %new_subnet.base.cidr,
                    "Discovered remote subnet via ipAddrTable"
                );

                match ctx.ops.create_subnet(&new_subnet, ctx.cancel).await {
                    Ok(created_subnet) => {
                        // Build an interface for the host on this subnet
                        let if_mac = snmp_if_entries
                            .iter()
                            .find(|e| e.if_index == entry.if_index)
                            .and_then(|e| e.if_phys_address);

                        host_data.add_ip_address(IPAddress::new(IPAddressBase {
                            network_id,
                            host_id: Uuid::nil(),
                            name: None,
                            subnet_id: created_subnet.id,
                            ip_address: *entry_ip,
                            // The device reporting its own `ifPhysAddress`: we asked it and it answered.
                            mac_address: if_mac.map(|m| {
                                MacEvidence::new(
                                    MacEvidenceValue(m),
                                    AttributeSource::Probe(ClientProbe::Snmp),
                                )
                            }),
                            position: 0,
                        }));

                        discovered_subnets.push(created_subnet);
                    }
                    Err(e) => {
                        tracing::warn!(
                            ip = %ip,
                            cidr = %new_subnet.base.cidr,
                            error = %e,
                            "Failed to create discovered subnet"
                        );
                    }
                }
            }
        }

        // --- Create loopback interface if this host has a SOFTWARE_LOOPBACK ifEntry ---
        let has_loopback_if_entry = snmp_if_entries
            .iter()
            .any(|e| e.if_type == Some(if_type::SOFTWARE_LOOPBACK));
        if has_loopback_if_entry {
            let loopback_subnet = Subnet::from_discovery(
                "lo".to_string(),
                &ipnetwork::IpNetwork::V4(
                    ipnetwork::Ipv4Network::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 8).unwrap(),
                ),
                network_id,
            );
            if let Some(loopback_subnet) = loopback_subnet {
                match ctx.ops.create_subnet(&loopback_subnet, ctx.cancel).await {
                    Ok(created_loopback) => {
                        host_data.add_ip_address(IPAddress::new(IPAddressBase {
                            network_id,
                            host_id: Uuid::nil(),
                            name: Some("lo".to_string()),
                            subnet_id: created_loopback.id,
                            ip_address: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                            mac_address: None,
                            position: 0,
                        }));
                    }
                    Err(e) => {
                        tracing::debug!(
                            error = %e,
                            "Failed to create loopback subnet for SNMP host"
                        );
                    }
                }
            }
        }

        // --- Discover remote hosts from ARP table ---
        // Only create hosts for ARP entries on SNMP-discovered remote subnets
        for arp_entry in &arp_entries {
            // Skip entries on the current scanning subnet
            if let Some(subnet) = scanning_subnet
                && subnet.base.cidr.contains(&arp_entry.ip_address)
            {
                continue;
            }

            // Find matching SNMP-discovered subnet
            let matching_subnet = discovered_subnets
                .iter()
                .find(|s| s.base.cidr.contains(&arp_entry.ip_address));

            if let Some(remote_subnet) = matching_subnet {
                let arp_interface = IPAddress::new(IPAddressBase {
                    network_id,
                    host_id: Uuid::nil(),
                    name: None,
                    subnet_id: remote_subnet.id,
                    ip_address: arp_entry.ip_address,
                    // A router's own ARP cache: a known speaker, about somebody else. Weaker than an
                    // ARP reply we solicited, and the distinction §6's minting rule turns on.
                    mac_address: Some(MacEvidence::new(
                        MacEvidenceValue(arp_entry.mac_address),
                        AttributeSource::ForwardingTable,
                    )),
                    position: 0,
                });

                // An ARP entry carries an address and nothing else. The host stays unnamed: the
                // display ladder titles it by that address without a copy in `name`.
                let arp_host = Host::new(HostBase {
                    network_id,
                    source: EntitySource::Discovery,
                    ..Default::default()
                });

                tracing::info!(
                    ip = %arp_entry.ip_address,
                    mac = %arp_entry.mac_address,
                    subnet = %remote_subnet.base.cidr,
                    "Discovered remote host via ARP table"
                );

                if let Err(e) = ctx
                    .ops
                    .create_host(
                        arp_host,
                        vec![arp_interface],
                        vec![],
                        vec![],
                        vec![],
                        vec![],
                        // ARP-discovered remote host has no ifTable of its own; nothing to prune.
                        true,
                        // ...and no neighbour data, so nothing to preserve against.
                        InterfaceDataComplete::default(),
                        ctx.cancel,
                    )
                    .await
                {
                    tracing::debug!(
                        ip = %arp_entry.ip_address,
                        error = %e,
                        "Failed to create ARP-discovered host"
                    );
                }
            }
        }

        // Shortfalls within SNMP are per-walk rather than per-collection — an incomplete ifTable
        // or neighbour walk is recorded above with the group it came from, which says far more
        // than a single count could. Reaching here means the collection itself ran to the end.
        Ok(Completeness::Complete)
    }
}

/// Perform a complete SNMP poll of a device.
/// Returns system info, interface table, and neighbor information.
#[allow(dead_code)]
pub async fn poll_device(
    ip: IpAddr,
    credential: &SnmpQueryCredential,
    port: u16,
) -> Result<(
    SystemInfo,
    Vec<IfTableEntry>,
    Vec<LldpNeighbor>,
    Vec<CdpNeighbor>,
)> {
    debug!("Starting SNMP poll of {}", ip);

    let mut session = create_session(ip, credential, port, SnmpContext::Default).await?;

    let system_info = timeout(SNMP_WALK_TIMEOUT, query_system_info(&mut session, ip))
        .await
        .map_err(|_| anyhow::anyhow!("System info query timeout"))??;

    let interfaces = timeout(SNMP_WALK_TIMEOUT, walk_if_table(&mut session, ip))
        .await
        .map_err(|_| anyhow::anyhow!("ifTable walk timeout"))?
        .map(|walk| walk.entries)
        .unwrap_or_default();

    let lldp_neighbors = timeout(SNMP_WALK_TIMEOUT, query_lldp_neighbors(&mut session, ip))
        .await
        .map(|r| r.map(|c| c.records))
        .unwrap_or(Ok(vec![]))
        .unwrap_or_default();

    let cdp_neighbors = timeout(SNMP_WALK_TIMEOUT, query_cdp_neighbors(&mut session, ip))
        .await
        .map(|r| r.map(|c| c.records))
        .unwrap_or(Ok(vec![]))
        .unwrap_or_default();

    debug!(
        "SNMP poll of {} complete: {} ip_addresses, {} LLDP neighbors, {} CDP neighbors",
        ip,
        interfaces.len(),
        lldp_neighbors.len(),
        cdp_neighbors.len()
    );

    Ok((system_info, interfaces, lldp_neighbors, cdp_neighbors))
}

#[cfg(test)]
mod tests;
