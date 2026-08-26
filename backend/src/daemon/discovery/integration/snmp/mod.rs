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
        hosts::r#impl::{
            base::{Host, HostBase},
            name::HostName,
        },
        interfaces::r#impl::base::{
            IfAdminStatus, IfOperStatus, Interface, InterfaceBase, InterfaceDataComplete, if_type,
        },
        ip_addresses::r#impl::base::{IPAddress, IPAddressBase},
        lldp::{LldpChassisId, LldpPortId},
        ports::r#impl::base::PortType,
        services::r#impl::patterns::ClientProbe,
        shared::types::entities::EntitySource,
        subnets::r#impl::base::Subnet,
    },
};

use super::{
    Checkpoint, Completeness, DiscoveryIntegration, IntegrationContext, IntegrationFailure,
    InterfaceViewScope, ProbeContext, ProbeFailure, ProbeSuccess,
};
use crate::daemon::discovery::service::ops::HostData;
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
                    convert_snmp_if_entry(entry, network_id, &[], &[], &[], &[], &no_vlan_uuids)
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

        // --- Hostname enrichment: use SNMP sysName as fallback if DNS didn't provide one ---
        if let Some(ref info) = system_info
            && let Some(ref sys_name) = info.sys_name
        {
            host_data.with_hostname_fallback(sys_name.clone());
        }

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
            host_data.with_mac_for_ip(ip, mac);
        }

        // --- Enrich host fields from SNMP system info ---
        if let Some(ref info) = system_info {
            if let Some(ref v) = info.sys_descr {
                host_data.with_sys_descr(v.clone());
            }
            if let Some(ref v) = info.sys_object_id {
                host_data.with_sys_object_id(v.clone());
            }
            if let Some(ref v) = info.sys_location {
                host_data.with_sys_location(v.clone());
            }
            if let Some(ref v) = info.sys_contact {
                host_data.with_sys_contact(v.clone());
            }
            if let Some(ref v) = info.sys_name {
                host_data.with_sys_name(v.clone());
            }
        }

        // --- Set chassis_id from LLDP local identity ---
        if let Some(ref local) = lldp_local
            && let Some(chassis) =
                LldpChassisId::from_snmp(local.chassis_id_subtype, &local.chassis_id_bytes)
        {
            // Same canonical form the server matches a *neighbor's* chassis ID against, so a
            // device whose chassis MAC appears on none of its ports is still identifiable.
            host_data.with_chassis_id(chassis.identifier());
        }

        // --- Add ENTITY-MIB hardware inventory ---
        if let Some(ref inventory) = device_inventory {
            if let Some(ref v) = inventory.manufacturer {
                host_data.with_manufacturer(v.clone());
            }
            if let Some(ref v) = inventory.model {
                host_data.with_model(v.clone());
            }
            if let Some(ref v) = inventory.serial_number {
                host_data.with_serial_number(v.clone());
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
                            mac_address: if_mac,
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
                    mac_address: Some(arp_entry.mac_address),
                    position: 0,
                });

                let mut arp_host = Host::new(HostBase {
                    network_id,
                    source: EntitySource::Discovery,
                    ..Default::default()
                });
                // An ARP entry carries an address and nothing else. Naming the host after it
                // beats the blank label these used to render as, and sits at the bottom of the
                // ladder so anything that later learns a real name replaces it.
                arp_host.base.apply_name(HostName::Ip(arp_entry.ip_address));

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

/// What placing a device's LLDP neighbours onto its interfaces produced.
///
/// Two different failures, because they call for different things. `unmatched` is a neighbour no
/// tier could place, which keeps its raw `lldpLocPortNum`; `dropped` is a neighbour that will
/// reach no interface at all and therefore contributes nothing — no chassis id is stored, no link
/// is drawn, and until this was counted the device simply looked as though it had no LLDP data.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LocalPortOutcome {
    /// Neighbours whose local port no tier could identify.
    pub unmatched: usize,
    /// Neighbours whose final index names no interface, or names one another neighbour already
    /// claimed. Every one of these is discarded whole by [`convert_snmp_if_entry`].
    pub dropped: usize,
}

/// Translate each LLDP neighbour's `local_port_index` from an `lldpLocPortNum` to the
/// device's real `ifIndex`, using `lldpLocPortTable` (`loc_ports`) resolved against the
/// interface table (`if_entries`). Neighbours whose port cannot be resolved keep their
/// original index. An empty `loc_ports` is identity — correct for devices where
/// `lldpLocPortNum == ifIndex` (e.g. Extreme VOSS) or that omit the table.
///
/// Both outcomes are counted because both are silent. An unmatched neighbour keeps its
/// `lldpLocPortNum`, which on a device where that is a separate namespace from `ifIndex` —
/// ExtremeXOS reports ports 1..N against ifIndexes 1001+ — attaches the link to whatever interface
/// happens to hold that index. Where it holds none, `convert_snmp_if_entry` attaches the neighbour
/// nowhere and the whole record is discarded: no `lldp_chassis_id` is ever written, so the device
/// contributes nothing and the server has nothing to resolve. The identity path is counted too —
/// returning zero there meant a device whose `lldpLocPortTable` was absent or unreadable dropped
/// every neighbour while raising no warning at all.
pub(crate) fn remap_lldp_local_ports(
    neighbors: &mut [LldpNeighbor],
    loc_ports: &HashMap<i32, LldpLocalPort>,
    if_entries: &[IfTableEntry],
) -> LocalPortOutcome {
    let mut outcome = LocalPortOutcome::default();

    // An empty table is the identity mapping, not a failure: devices where `lldpLocPortNum ==
    // ifIndex` (Extreme VOSS, most vendors) legitimately omit it. It still has to be checked —
    // identity is only correct where the number *is* an ifIndex.
    if !loc_ports.is_empty() {
        // Built once for the whole device rather than per neighbour, and deliberately only for
        // addresses belonging to exactly one interface. See [`unique_interface_macs`].
        let macs = unique_interface_macs(if_entries);
        for neighbor in neighbors.iter_mut() {
            let port = neighbor.local_port_index;
            match resolve_lldp_local_port(port, loc_ports, if_entries, &macs) {
                Some((if_index, evidence)) => {
                    tracing::debug!(
                        local_port = port,
                        if_index,
                        ?evidence,
                        "Matched an LLDP local port to an interface"
                    );
                    neighbor.local_port_index = if_index;
                }
                None => {
                    // The evidence, not just the failure. This is the line that decides whether the
                    // next unmatched switch needs another walk from its owner: it names what the
                    // device offered and therefore which tier would have to grow to place it.
                    let entry = loc_ports.get(&port);
                    tracing::debug!(
                        local_port = port,
                        subtype = ?entry.and_then(|e| e.port_id_subtype),
                        port_id = ?entry.and_then(|e| e.port_id.as_deref()),
                        port_id_mac = ?entry.and_then(|e| e.port_id_mac),
                        port_desc = ?entry.and_then(|e| e.port_desc.as_deref()),
                        "No interface matched an LLDP local port"
                    );
                    outcome.unmatched += 1;
                }
            }
        }
    }

    outcome.dropped = count_dropped_neighbours(neighbors, loc_ports, if_entries);
    outcome
}

/// Count the neighbours that will reach no interface, naming each one's evidence.
///
/// Mirrors the attachment rule in [`convert_snmp_if_entry`] exactly — first neighbour whose
/// `local_port_index` equals an interface's `if_index` — so "counted as dropped" and "actually
/// dropped" cannot drift apart. The evidence line carries the far end's own identity as well as
/// the local-port columns, because on the identity path there is no `lldpLocPortTable` row to
/// describe and the neighbour's chassis is the only thing that names what was lost.
pub(crate) fn count_dropped_neighbours(
    neighbors: &[LldpNeighbor],
    loc_ports: &HashMap<i32, LldpLocalPort>,
    if_entries: &[IfTableEntry],
) -> usize {
    let if_indexes: HashSet<i32> = if_entries.iter().map(|e| e.if_index).collect();
    let mut claimed: HashSet<i32> = HashSet::new();
    let mut dropped = 0;

    for neighbor in neighbors {
        let port = neighbor.local_port_index;
        let reason = if !if_indexes.contains(&port) {
            "no interface on the device has this ifIndex"
        } else if !claimed.insert(port) {
            "another neighbour on the same port was recorded first"
        } else {
            continue;
        };

        let entry = loc_ports.get(&port);
        tracing::debug!(
            local_port = port,
            reason,
            subtype = ?entry.and_then(|e| e.port_id_subtype),
            port_id = ?entry.and_then(|e| e.port_id.as_deref()),
            port_id_mac = ?entry.and_then(|e| e.port_id_mac),
            port_desc = ?entry.and_then(|e| e.port_desc.as_deref()),
            remote_chassis_subtype = ?neighbor.remote_chassis_id_subtype,
            remote_port_subtype = ?neighbor.remote_port_id_subtype,
            remote_port_desc = ?neighbor.remote_port_desc.as_deref(),
            remote_sys_name = ?neighbor.remote_sys_name.as_deref(),
            "Discarding an LLDP neighbour that reaches no interface"
        );
        dropped += 1;
    }

    dropped
}

/// Which column matched, for the log line that has to explain a device nothing matched on.
///
/// Ordered as the tiers are tried: an identifier that names the interface outright beats one that
/// has to be matched by shape, and both beat free text. Both name tiers come before both shape
/// tiers, whichever column they read — a whole name is more than the device had to tell us, and a
/// fragment that happens to match is less, so the column they arrive in does not outrank that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalPortEvidence {
    /// `lldpLocPortIdSubtype = 2` — the id is the ifIndex, and the device has one by that number.
    InterfaceIndex,
    /// `lldpLocPortIdSubtype = 3` — the id is a MAC held by exactly one interface.
    UniqueMac,
    /// `lldpLocPortId` equals an ifName, ifDescr or ifAlias.
    PortIdName,
    /// `lldpLocPortDesc` equals an ifName, ifDescr or ifAlias.
    PortDescName,
    /// `lldpLocPortId` is the tail of one interface's ifName or ifDescr, at a slot boundary.
    PortIdSuffix,
    /// One word of `lldpLocPortDesc` equals an ifName or ifDescr, and only one interface's.
    PortDescWord,
}

/// The interfaces whose `ifPhysAddress` identifies them on their own.
///
/// A MAC is only evidence of *which* port when the device gives each port a different one.
/// Westermo does; the D-Link DGS and TP-Link switches in GH #668 report the chassis address on
/// every interface, and matching on it there would collapse every neighbour onto one port —
/// worse than leaving them unresolved, because the resulting map looks complete. So an address
/// that appears more than once is dropped rather than arbitrated, and those devices fall through
/// to the description tiers.
///
/// The all-zero address is dropped for the same reason: it is what firmware reports for an
/// interface that has no hardware address, not an identity.
pub(crate) fn unique_interface_macs(if_entries: &[IfTableEntry]) -> HashMap<MacAddress, i32> {
    let unset = MacAddress::new([0; 6]);
    let mut by_mac: HashMap<MacAddress, Option<i32>> = HashMap::new();
    for e in if_entries {
        let Some(mac) = e.if_phys_address.filter(|m| *m != unset) else {
            continue;
        };
        by_mac
            .entry(mac)
            .and_modify(|slot| *slot = None)
            .or_insert(Some(e.if_index));
    }
    by_mac
        .into_iter()
        .filter_map(|(mac, if_index)| if_index.map(|i| (mac, i)))
        .collect()
}

/// Resolve a single `lldpLocPortNum` to an `ifIndex`, and say what matched it. Returns `None` to
/// keep the original value (no confident match).
///
/// Tiered most-specific-first, because the columns disagree in practice and the cost of a wrong
/// answer is a link drawn against the wrong port. Every tier is something a real device needed:
/// ExtremeXOS numbers its LLDP ports separately from its interfaces, Westermo identifies every
/// port by MAC and names it only in the description, and the description is free text on a device
/// that is under no obligation to make it parseable.
fn resolve_lldp_local_port(
    local_port_num: i32,
    loc_ports: &HashMap<i32, LldpLocalPort>,
    if_entries: &[IfTableEntry],
    unique_macs: &HashMap<MacAddress, i32>,
) -> Option<(i32, LocalPortEvidence)> {
    let entry = loc_ports.get(&local_port_num)?;

    // interfaceIndex(2): the port id is literally the ifIndex — but only if the device has an
    // interface by that number. Returning the advertised integer unchecked put neighbours on a
    // port that does not exist, where `count_dropped_neighbours` discards them whole and the
    // switch reads as having no LLDP at all; a Dell OS10 numbers its LLDP ports past 568 against
    // 23 interfaces, so an unchecked answer here is not a near miss (GH #685). Falling through
    // gives the name and description tiers, which know the OS10 port names, their turn.
    if entry.port_id_subtype == Some(2)
        && let Some(id) = entry.port_id.as_deref()
        && let Ok(idx) = id.trim().parse::<i32>()
        && if_entries.iter().any(|e| e.if_index == idx)
    {
        return Some((idx, LocalPortEvidence::InterfaceIndex));
    }

    // macAddress(3): the port id is the port's own hardware address, in raw octets. Only usable
    // where that address belongs to one interface — see `unique_interface_macs`.
    if entry.port_id_subtype == Some(3)
        && let Some(mac) = entry.port_id_mac
        && let Some(&if_index) = unique_macs.get(&mac)
    {
        return Some((if_index, LocalPortEvidence::UniqueMac));
    }

    let named = |text: &str, evidence| {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        // Exact match against ifName / ifDescr / ifAlias (VOSS: "1/1" == ifName "1/1"). ifAlias is
        // included to match the server's ladder in `server::lldp::resolver`, which added it
        // for Westermo WeOS — the daemon holding a narrower rule than the server meant the two
        // could place the same neighbour on different ports.
        if_entries
            .iter()
            .find(|e| {
                e.if_name.as_deref() == Some(text)
                    || e.if_descr.as_deref() == Some(text)
                    || e.if_alias.as_deref() == Some(text)
            })
            .map(|e| (e.if_index, evidence))
    };

    if let Some(id) = entry.port_id.as_deref()
        && let Some(hit) = named(id, LocalPortEvidence::PortIdName)
    {
        return Some(hit);
    }

    // An exact name in the description outranks a partial match on the id. Both are the same
    // question — which interface is this? — answered with different amounts of evidence, and the
    // suffix tier answers it from a fragment. A Dell OS10 breakout port advertising the bare id
    // "4" alongside the description "mgmt1/1/1" ends at the boundary in `ethernet1/1/4` and
    // nowhere else, so the fragment is unambiguous and wrong: it names a port on the front panel
    // while the device is telling us, in full, which port it means (GH #685).
    let desc = entry.port_desc.as_deref();
    if let Some(hit) = desc.and_then(|d| named(d, LocalPortEvidence::PortDescName)) {
        return Some(hit);
    }

    // Suffix match for vendors whose lldpLocPortId drops the slot prefix (EXOS: id
    // "4" vs ifName "1:4"). Anchor on a ':' or '/' boundary so "4" does not match
    // "14".
    //
    // Only when the boundary names one interface. The anchor characters mean different things to
    // different vendors — on EXOS ':' separates slot from port, on Dell OS10 it separates a port
    // from its breakout lane — so on a switch carrying both `ethernet1/1/1` and
    // `ethernet1/1/14:1` the id "1" ends at a boundary in three places at once, and taking the
    // first left a neighbour bound to a lane of an unrelated port, or to `mgmt1/1/1`, with
    // `PortIdSuffix` recorded as though it were evidence. Same rule as the description-word tier
    // below: an id matching two interfaces is evidence of neither.
    if let Some(id) = entry.port_id.as_deref() {
        let id = id.trim();
        if !id.is_empty() {
            let colon = format!(":{id}");
            let slash = format!("/{id}");
            let ends_at_boundary = |name: Option<&str>| {
                name.is_some_and(|n| n.ends_with(&colon) || n.ends_with(&slash))
            };
            let matched: Vec<i32> = if_entries
                .iter()
                .filter(|e| {
                    ends_at_boundary(e.if_name.as_deref())
                        || ends_at_boundary(e.if_descr.as_deref())
                })
                .map(|e| e.if_index)
                .collect();
            if let [only] = matched[..] {
                return Some((only, LocalPortEvidence::PortIdSuffix));
            }
        }
    }

    let desc = desc?;

    // The description is prose, and the interface name may be one word of it — Westermo sends
    // "100-T eth10" for the port whose ifName is "eth10". Take a word only when it identifies a
    // single interface: a description matching two of them is not evidence of either.
    let mut matched: Vec<i32> = Vec::new();
    for word in desc.split_whitespace() {
        for e in if_entries {
            if (e.if_name.as_deref() == Some(word) || e.if_descr.as_deref() == Some(word))
                && !matched.contains(&e.if_index)
            {
                matched.push(e.if_index);
            }
        }
    }
    if let [only] = matched[..] {
        return Some((only, LocalPortEvidence::PortDescWord));
    }

    None
}

/// Convert SNMP ifTable entry to Interface entity with LLDP/CDP/FDB neighbor data.
/// Uses Uuid::nil() for host_id as placeholder - server will set correct host_id.
fn convert_snmp_if_entry(
    entry: &IfTableEntry,
    network_id: Uuid,
    lldp_neighbors: &[LldpNeighbor],
    cdp_neighbors: &[CdpNeighbor],
    bridge_fdb: &[BridgeFdbEntry],
    port_vlan_membership: &[PortVlanMembership],
    vlan_number_to_uuid: &std::collections::HashMap<u16, Uuid>,
) -> Interface {
    // Find LLDP neighbor data for this port (match by local_port_index == if_index)
    let lldp_neighbor = lldp_neighbors
        .iter()
        .find(|n| n.local_port_index == entry.if_index);

    // Find CDP neighbor data for this port
    let cdp_neighbor = cdp_neighbors
        .iter()
        .find(|n| n.local_port_index == entry.if_index);

    // Convert LLDP chassis ID using subtype + raw bytes via from_snmp()
    let lldp_chassis_id = lldp_neighbor.and_then(|n| {
        let subtype = n.remote_chassis_id_subtype?;
        let bytes = n.remote_chassis_id_bytes.as_ref()?;
        LldpChassisId::from_snmp(subtype, bytes)
    });

    // Convert LLDP port ID using subtype + raw bytes via from_snmp()
    let lldp_port_id = lldp_neighbor.and_then(|n| {
        let subtype = n.remote_port_id_subtype?;
        let bytes = n.remote_port_id_bytes.as_ref()?;
        LldpPortId::from_snmp(subtype, bytes)
    });

    // Find VLAN membership for this port
    let vlan_membership = port_vlan_membership
        .iter()
        .find(|m| m.if_index == entry.if_index);

    // Collect learned MACs from bridge FDB for this port.
    // Single-MAC ports are used for neighbor resolution server-side;
    // multi-MAC ports indicate uplinks where LLDP/CDP is the better source
    // for direct neighbor identification.
    let fdb_macs: Vec<String> = bridge_fdb
        .iter()
        .filter(|fdb| fdb.if_index == Some(entry.if_index) && fdb.status == 3)
        .map(|fdb| fdb.mac_address.to_string())
        .collect();

    Interface::new(InterfaceBase {
        host_id: Uuid::nil(), // Placeholder - server will set correct host_id
        network_id,
        if_index: entry.if_index,
        if_descr: entry.if_descr.clone().unwrap_or_default(),
        if_name: entry.if_name.clone(),
        if_alias: entry.if_alias.clone(),
        if_type: entry.if_type.unwrap_or(1), // 1 = "other"
        speed_bps: entry.if_speed.map(|s| s as i64),
        admin_status: IfAdminStatus::from(entry.if_admin_status.unwrap_or(1)),
        oper_status: IfOperStatus::from(entry.if_oper_status.unwrap_or(1)),
        mac_address: entry.if_phys_address, // MAC from SNMP ifPhysAddress
        ip_address_id: None,                // Linked server-side via MAC matching
        neighbor: None,                     // Resolved server-side from LLDP/CDP data
        // Stamped server-side from the evidence carried in this payload, on ingest.
        neighbor_seen_at: None,
        // LLDP raw data
        lldp_chassis_id,
        lldp_port_id,
        lldp_sys_name: lldp_neighbor.and_then(|n| n.remote_sys_name.clone()),
        lldp_port_desc: lldp_neighbor.and_then(|n| n.remote_port_desc.clone()),
        lldp_mgmt_addr: lldp_neighbor.and_then(|n| n.remote_mgmt_addr),
        lldp_sys_desc: lldp_neighbor.and_then(|n| n.remote_sys_desc.clone()),
        // CDP raw data
        cdp_device_id: cdp_neighbor.and_then(|n| n.remote_device_id.clone()),
        cdp_port_id: cdp_neighbor.and_then(|n| n.remote_port_id.clone()),
        cdp_platform: cdp_neighbor.and_then(|n| n.remote_platform.clone()),
        cdp_address: cdp_neighbor.and_then(|n| n.remote_address),
        // Bridge FDB data
        fdb_macs: if fdb_macs.is_empty() {
            None
        } else {
            Some(fdb_macs)
        },
        // VLAN data: resolved to entity UUIDs by caller via vlan_number_to_uuid mapping
        native_vlan_id: vlan_membership
            .and_then(|m| m.native_vlan)
            .and_then(|vid| vlan_number_to_uuid.get(&vid).copied()),
        vlan_ids: vlan_membership
            .map(|m| {
                m.tagged_vlans
                    .iter()
                    .filter_map(|vid| vlan_number_to_uuid.get(vid).copied())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty()),
    })
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
mod tests {
    use super::values::{value_to_i32, value_to_mac, value_to_string};
    use snmp2::Value;

    /// The interface set is persisted as soon as the ifTable walk finishes, before the
    /// neighbour/FDB/VLAN queries have run — so it is built with no enrichment available.
    /// Those bare interfaces still have to be complete, usable entities (the host is created
    /// from them if a later query hangs), carrying every ifTable field and simply no
    /// LLDP/CDP/FDB/VLAN data.
    #[test]
    fn interfaces_built_without_enrichment_keep_their_iftable_identity() {
        use super::*;

        let entry = types::IfTableEntry {
            if_index: 7,
            if_descr: Some("Port 7".to_string()),
            if_name: Some("swp7".to_string()),
            if_type: Some(6),
            if_speed: Some(1_000_000_000),
            if_admin_status: Some(1),
            if_oper_status: Some(1),
            ..Default::default()
        };
        let network_id = Uuid::new_v4();

        let interface = convert_snmp_if_entry(
            &entry,
            network_id,
            &[],
            &[],
            &[],
            &[],
            &std::collections::HashMap::new(),
        );

        // ifTable data survives the enrichment-free conversion.
        assert_eq!(interface.base.if_index, 7);
        assert_eq!(interface.base.if_descr, "Port 7");
        assert_eq!(interface.base.if_name.as_deref(), Some("swp7"));
        assert_eq!(interface.base.if_type, 6);
        assert_eq!(interface.base.speed_bps, Some(1_000_000_000));
        assert_eq!(interface.base.network_id, network_id);

        // Enrichment that hasn't been collected yet is absent, not fabricated.
        assert!(interface.base.lldp_chassis_id.is_none());
        assert!(interface.base.cdp_device_id.is_none());
        assert!(interface.base.fdb_macs.is_none());
        assert!(interface.base.native_vlan_id.is_none());
        assert!(interface.base.vlan_ids.is_none());
    }

    #[test]
    fn test_value_to_string() {
        let value = Value::OctetString(b"test string");
        assert_eq!(value_to_string(&value), Some("test string".to_string()));
    }

    #[test]
    fn test_value_to_i32() {
        let value = Value::Integer(42);
        assert_eq!(value_to_i32(&value), Some(42));
    }

    #[test]
    fn test_value_to_mac() {
        let mac_bytes: [u8; 6] = [0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34];
        let value = Value::OctetString(&mac_bytes);
        let mac = value_to_mac(&value).unwrap();
        assert_eq!(mac.bytes(), [0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34]);
    }

    #[test]
    fn test_convert_snmp_if_entry_with_vlan_data() {
        use super::convert_snmp_if_entry;
        use super::types::{IfTableEntry, PortVlanMembership};
        use uuid::Uuid;

        let entry = IfTableEntry {
            if_index: 5,
            if_descr: Some("GigabitEthernet0/5".to_string()),
            ..Default::default()
        };

        let membership = vec![
            PortVlanMembership {
                if_index: 5,
                native_vlan: Some(10),
                tagged_vlans: vec![20, 30],
            },
            PortVlanMembership {
                if_index: 7,
                native_vlan: Some(20),
                tagged_vlans: vec![],
            },
        ];

        let result = convert_snmp_if_entry(
            &entry,
            Uuid::nil(),
            &[],
            &[],
            &[],
            &membership,
            &std::collections::HashMap::new(),
        );

        assert_eq!(result.base.native_vlan_id, None);
        assert_eq!(result.base.vlan_ids, None);
    }

    #[test]
    fn test_convert_snmp_if_entry_no_vlan_data() {
        use super::convert_snmp_if_entry;
        use super::types::IfTableEntry;
        use uuid::Uuid;

        let entry = IfTableEntry {
            if_index: 3,
            if_descr: Some("Loopback0".to_string()),
            ..Default::default()
        };

        let result = convert_snmp_if_entry(
            &entry,
            Uuid::nil(),
            &[],
            &[],
            &[],
            &[],
            &std::collections::HashMap::new(),
        );

        assert_eq!(result.base.native_vlan_id, None);
        assert_eq!(result.base.vlan_ids, None);
    }

    #[test]
    fn test_convert_snmp_if_entry_empty_tagged_vlans() {
        use super::convert_snmp_if_entry;
        use super::types::{IfTableEntry, PortVlanMembership};
        use uuid::Uuid;

        let entry = IfTableEntry {
            if_index: 1,
            if_descr: Some("FastEthernet0/1".to_string()),
            ..Default::default()
        };

        // Access port: native VLAN only, no tagged VLANs
        let membership = vec![PortVlanMembership {
            if_index: 1,
            native_vlan: Some(10),
            tagged_vlans: vec![],
        }];

        let result = convert_snmp_if_entry(
            &entry,
            Uuid::nil(),
            &[],
            &[],
            &[],
            &membership,
            &std::collections::HashMap::new(),
        );

        assert_eq!(result.base.native_vlan_id, None);
        // Empty tagged_vlans should be stored as None (filtered)
        assert_eq!(result.base.vlan_ids, None);
    }

    // --- LLDP local-port remap (Issue 2: ExtremeXOS vs VOSS) ---

    use super::types::{IfTableEntry, LldpLocalPort, LldpNeighbor};

    /// Minimal LldpNeighbor carrying only a local-port index + a marker sys name.
    fn lldp_neighbor(local_port_index: i32, sys_name: &str) -> LldpNeighbor {
        LldpNeighbor {
            local_port_index,
            remote_chassis_id_subtype: None,
            remote_chassis_id_bytes: None,
            remote_port_id_subtype: None,
            remote_port_id_bytes: None,
            remote_port_desc: None,
            remote_sys_name: Some(sys_name.to_string()),
            remote_sys_desc: None,
            remote_mgmt_addr: None,
        }
    }

    fn if_entry(if_index: i32, if_name: &str) -> IfTableEntry {
        IfTableEntry {
            if_index,
            if_name: Some(if_name.to_string()),
            ..Default::default()
        }
    }

    fn loc_port(subtype: u8, id: &str) -> LldpLocalPort {
        LldpLocalPort {
            port_id_subtype: Some(subtype),
            port_id: Some(id.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_remap_lldp_exos_suffix_match() {
        use super::remap_lldp_local_ports;
        // ExtremeXOS X435: lldpRemTable local-port is an lldpLocPortNum (4, 11) in a
        // 1..N space; real ifIndex is 1001+, ifName "1:N". lldpLocPortId is "N",
        // subtype interfaceName(5) — must suffix-match against ifName "1:N".
        let if_entries = [
            if_entry(1001, "1:1"),
            if_entry(1004, "1:4"),
            if_entry(1011, "1:11"),
        ];
        let mut loc_ports = std::collections::HashMap::new();
        loc_ports.insert(4, loc_port(5, "4"));
        loc_ports.insert(11, loc_port(5, "11"));

        let mut neighbors = vec![lldp_neighbor(4, "peer-a"), lldp_neighbor(11, "peer-b")];
        remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        assert_eq!(neighbors[0].local_port_index, 1004);
        assert_eq!(neighbors[1].local_port_index, 1011);
    }

    #[test]
    fn test_remap_lldp_voss_exact_match_identity() {
        use super::remap_lldp_local_ports;
        // Extreme VOSS: lldpLocPortNum == ifIndex and lldpLocPortId ("1/1") matches
        // ifName exactly, so the resolved ifIndex equals the original index.
        let if_entries = [if_entry(192, "1/1"), if_entry(193, "1/2")];
        let mut loc_ports = std::collections::HashMap::new();
        loc_ports.insert(192, loc_port(5, "1/1"));
        loc_ports.insert(193, loc_port(5, "1/2"));

        let mut neighbors = vec![lldp_neighbor(192, "peer")];
        remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        assert_eq!(neighbors[0].local_port_index, 192);
    }

    #[test]
    fn test_remap_lldp_no_loc_table_is_identity() {
        use super::remap_lldp_local_ports;
        // No lldpLocPortTable (e.g. devices that report lldpLocPortNum == ifIndex):
        // indices are left untouched so existing behaviour is preserved.
        let if_entries = [if_entry(5, "Gi0/5")];
        let empty = std::collections::HashMap::new();
        let mut neighbors = vec![lldp_neighbor(5, "peer")];
        let outcome = remap_lldp_local_ports(&mut neighbors, &empty, &if_entries);
        assert_eq!(neighbors[0].local_port_index, 5);
        assert_eq!(outcome, super::LocalPortOutcome::default());
    }

    /// The identity path is only correct where the local port number *is* an ifIndex. Where the
    /// device numbers its LLDP ports separately and serves no `lldpLocPortTable` — or served one
    /// the walk could not read — every neighbour lands on an index no interface holds and is
    /// discarded by `convert_snmp_if_entry` without a word. Returning zero here is what let a
    /// switch report neighbours all day and appear to have none.
    #[test]
    fn an_absent_local_port_table_over_a_separate_numbering_drops_every_neighbour() {
        use super::remap_lldp_local_ports;

        let if_entries = [if_entry(1001, "1:1"), if_entry(1002, "1:2")];
        let empty = std::collections::HashMap::new();
        let mut neighbors = vec![lldp_neighbor(1, "peer-a"), lldp_neighbor(2, "peer-b")];

        let outcome = remap_lldp_local_ports(&mut neighbors, &empty, &if_entries);

        assert_eq!(outcome.dropped, 2);
        assert_eq!(
            outcome.unmatched, 0,
            "no tier ran, so nothing failed to match — the loss is the drop"
        );
    }

    /// `convert_snmp_if_entry` attaches the first neighbour whose index matches and no more, so a
    /// second one on the same port is lost as completely as one on no port. Counting only the
    /// index misses would report this device as clean.
    #[test]
    fn a_second_neighbour_on_one_port_is_counted_as_dropped() {
        use super::remap_lldp_local_ports;

        let if_entries = [if_entry(3, "Gi0/3")];
        let empty = std::collections::HashMap::new();
        let mut neighbors = vec![lldp_neighbor(3, "phone"), lldp_neighbor(3, "laptop")];

        let outcome = remap_lldp_local_ports(&mut neighbors, &empty, &if_entries);

        assert_eq!(outcome.dropped, 1);
    }

    #[test]
    fn test_remap_lldp_interface_index_subtype() {
        use super::remap_lldp_local_ports;
        // lldpLocPortId subtype interfaceIndex(2): the id is literally the ifIndex.
        let if_entries = [if_entry(1007, "1:7")];
        let mut loc_ports = std::collections::HashMap::new();
        loc_ports.insert(7, loc_port(2, "1007"));
        let mut neighbors = vec![lldp_neighbor(7, "peer")];
        remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);
        assert_eq!(neighbors[0].local_port_index, 1007);
    }

    #[test]
    fn test_remap_then_convert_attaches_exos_neighbor() {
        use super::{convert_snmp_if_entry, remap_lldp_local_ports};
        use uuid::Uuid;
        // End-to-end at the convert layer: after remap, the EXOS neighbour attaches
        // to the correct interface (which it would NOT before the fix, since
        // local_port_index 4 != ifIndex 1004).
        let if_entries = [if_entry(1004, "1:4")];
        let mut loc_ports = std::collections::HashMap::new();
        loc_ports.insert(4, loc_port(5, "4"));

        let mut neighbors = vec![lldp_neighbor(4, "switch-peer")];
        remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        let result = convert_snmp_if_entry(
            &if_entries[0],
            Uuid::nil(),
            &neighbors,
            &[],
            &[],
            &[],
            &std::collections::HashMap::new(),
        );
        assert_eq!(result.base.lldp_sys_name, Some("switch-peer".to_string()));
    }

    // --- macAddress(3) local ports (Westermo industrial switches) ---

    use std::collections::HashMap;

    fn mac(last: u8) -> mac_address::MacAddress {
        mac_address::MacAddress::new([0x02, 0x00, 0x00, 0x00, 0x00, last])
    }

    fn if_entry_with_mac(
        if_index: i32,
        if_name: &str,
        phys: mac_address::MacAddress,
    ) -> IfTableEntry {
        IfTableEntry {
            if_phys_address: Some(phys),
            ..if_entry(if_index, if_name)
        }
    }

    /// A switch reporting `lldpLocPortIdSubtype = 3` gives each port's own MAC as the id, in raw
    /// octets. That is not text, so the id never survived being read as a string and the port had
    /// nothing to match on — every neighbour on the device stayed unresolved.
    #[test]
    fn a_port_identified_by_its_own_mac_resolves_to_that_interface() {
        use super::remap_lldp_local_ports;

        let if_entries = [
            if_entry_with_mac(1, "eth1", mac(0xE1)),
            if_entry_with_mac(2, "eth2", mac(0xE2)),
        ];
        let mut loc_ports = HashMap::new();
        loc_ports.insert(
            19,
            LldpLocalPort {
                port_id_subtype: Some(3),
                port_id_mac: Some(mac(0xE1)),
                ..Default::default()
            },
        );

        let mut neighbors = vec![lldp_neighbor(19, "peer")];
        let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        assert_eq!(outcome.unmatched, 0);
        assert_eq!(neighbors[0].local_port_index, 1);
    }

    /// The D-Link DGS and TP-Link switches in GH #668 report the chassis MAC on every interface.
    /// Matching on it would put every neighbour on whichever port won the lookup — a map that
    /// looks complete and is wrong. The tier must decline and let a later one answer.
    #[test]
    fn a_mac_shared_by_every_interface_resolves_through_the_description_instead() {
        use super::remap_lldp_local_ports;

        let shared = mac(0xAA);
        let if_entries = [
            if_entry_with_mac(1, "eth1", shared),
            if_entry_with_mac(2, "eth2", shared),
        ];
        let mut loc_ports = HashMap::new();
        loc_ports.insert(
            19,
            LldpLocalPort {
                port_id_subtype: Some(3),
                port_id_mac: Some(shared),
                port_desc: Some("1000-LX eth2".to_string()),
                ..Default::default()
            },
        );

        let mut neighbors = vec![lldp_neighbor(19, "peer")];
        let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        assert_eq!(outcome.unmatched, 0);
        assert_eq!(
            neighbors[0].local_port_index, 2,
            "the description names the port; the shared MAC names nothing"
        );
    }

    /// The reference case. Local port numbers run 10..19 against interfaces eth10 down to eth1,
    /// so there is no arithmetic to exploit and the description is the only authority — and it
    /// carries the media type in front of the name.
    #[test]
    fn local_port_numbers_that_run_backwards_map_through_the_description() {
        use super::remap_lldp_local_ports;

        let if_entries: Vec<IfTableEntry> = (1..=10)
            .map(|n| if_entry_with_mac(n, &format!("eth{n}"), mac(0xE0 + n as u8)))
            .collect();
        // Port 10 is eth10 and each port after it counts the interfaces back down.
        let mut loc_ports = HashMap::new();
        for port in 10..=19i32 {
            let interface = 20 - port;
            loc_ports.insert(
                port,
                LldpLocalPort {
                    port_id_subtype: Some(3),
                    // A distinct MAC per port, as this vendor sends — but one that belongs to no
                    // interface, so only the description can place it.
                    port_id_mac: Some(mac(0x70 + port as u8)),
                    port_desc: Some(format!("100-T eth{interface}")),
                    ..Default::default()
                },
            );
        }

        let mut neighbors = vec![
            lldp_neighbor(11, "peer-a"),
            lldp_neighbor(19, "peer-b"),
            lldp_neighbor(16, "peer-c"),
        ];
        let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        assert_eq!(outcome.unmatched, 0);
        assert_eq!(
            neighbors
                .iter()
                .map(|n| n.local_port_index)
                .collect::<Vec<_>>(),
            vec![9, 1, 4]
        );
    }

    /// A description word that names two interfaces is not evidence of either. Leaving the
    /// neighbour unresolved is the honest outcome — it is counted and warned about, where a
    /// wrong port would be neither.
    #[test]
    fn a_description_matching_two_interfaces_resolves_to_neither() {
        use super::remap_lldp_local_ports;

        // Two interfaces answering to the same name across ifName and ifDescr.
        let if_entries = [
            if_entry(1, "eth1"),
            IfTableEntry {
                if_index: 2,
                if_descr: Some("eth1".to_string()),
                ..Default::default()
            },
        ];
        let mut loc_ports = HashMap::new();
        loc_ports.insert(
            11,
            LldpLocalPort {
                port_id_subtype: Some(3),
                port_desc: Some("100-T eth1".to_string()),
                ..Default::default()
            },
        );

        let mut neighbors = vec![lldp_neighbor(11, "peer")];
        let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        assert_eq!(outcome.unmatched, 1);
        assert_eq!(
            neighbors[0].local_port_index, 11,
            "an unresolved neighbour keeps its local port number rather than guessing"
        );
        assert_eq!(
            outcome.dropped, 1,
            "port 11 is no interface's ifIndex, so the neighbour reaches nothing at all"
        );
    }

    // --- Dell OS10 breakout ports (GH #685) ---

    /// A Dell PowerSwitch S4112T-ON as the reporter's switch is configured: port 14 broken out
    /// into three lanes, so the interface names carry both a `/` and a `:`, and `mgmt1/1/1`
    /// repeats the `/1` the lanes end on. Breakout lanes come before the management port because
    /// OS10 numbers its ethernet interfaces below it.
    fn dell_os10_if_entries() -> Vec<IfTableEntry> {
        let mut entries = vec![
            if_entry(15, "ethernet1/1/14:1"),
            if_entry(16, "ethernet1/1/14:2"),
            if_entry(17, "ethernet1/1/14:3"),
        ];
        entries.extend((1..=13).map(|n| if_entry(n + 1, &format!("ethernet1/1/{n}"))));
        entries.push(if_entry(1, "mgmt1/1/1"));
        entries
    }

    fn loc_port_named(subtype: u8, id: &str, desc: &str) -> LldpLocalPort {
        LldpLocalPort {
            port_desc: Some(desc.to_string()),
            ..loc_port(subtype, id)
        }
    }

    /// The mapping the reporter published, end to end: local ports 4, 568, 569 and 570 reach
    /// `mgmt1/1/1` and the three lanes of port 14, and nothing else. `lldpLocPortNum` is a
    /// separate namespace here — it runs past 568 against 23 interfaces — so every one of these
    /// has to come from the port table rather than from the number itself.
    #[test]
    fn dell_os10_breakout_neighbours_reach_the_ports_the_switch_names() {
        use super::remap_lldp_local_ports;
        let if_entries = dell_os10_if_entries();
        let mut loc_ports = HashMap::new();
        loc_ports.insert(4, loc_port(5, "mgmt1/1/1"));
        loc_ports.insert(568, loc_port(5, "ethernet1/1/14:1"));
        loc_ports.insert(569, loc_port(5, "ethernet1/1/14:2"));
        loc_ports.insert(570, loc_port(5, "ethernet1/1/14:3"));

        let mut neighbors = vec![
            lldp_neighbor(570, "TAMMIERENEW"),
            lldp_neighbor(4, "unnamed-host"),
            lldp_neighbor(568, "EVILCORP"),
            lldp_neighbor(569, "VIRTUALPC"),
        ];
        let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        assert_eq!(outcome, super::LocalPortOutcome::default());
        let placed: Vec<(&str, i32)> = neighbors
            .iter()
            .map(|n| (n.remote_sys_name.as_deref().unwrap(), n.local_port_index))
            .collect();
        assert_eq!(
            placed,
            vec![
                ("TAMMIERENEW", 17),
                ("unnamed-host", 1),
                ("EVILCORP", 15),
                ("VIRTUALPC", 16),
            ]
        );
    }

    /// The suffix tier anchors on `:` and `/`, and on this switch a bare port id ends at one in
    /// three places at once — `mgmt1/1/1`, `ethernet1/1/1` and the first lane of port 14 all
    /// qualify for the id "1". Taking the first match placed the neighbour on whichever interface
    /// the ifTable happened to list first and recorded `PortIdSuffix` as though that were
    /// evidence. An id matching three interfaces is evidence of none of them, so the walk falls
    /// through to the description, which names one port and only one.
    #[test]
    fn a_port_id_ending_at_three_boundaries_defers_to_the_description() {
        use super::remap_lldp_local_ports;
        let if_entries = dell_os10_if_entries();
        let mut loc_ports = HashMap::new();
        loc_ports.insert(4, loc_port_named(5, "1", "mgmt1/1/1"));

        let mut neighbors = vec![lldp_neighbor(4, "peer")];
        let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        assert_eq!(outcome, super::LocalPortOutcome::default());
        assert_eq!(
            neighbors[0].local_port_index, 1,
            "the description names mgmt1/1/1; the ambiguous suffix must not outrank it"
        );
    }

    /// An unambiguous suffix match can still be the wrong port. The management port advertising
    /// the bare id "4" ends at a boundary in `ethernet1/1/4` and nowhere else, so uniqueness does
    /// not save it — but the same row's description says `mgmt1/1/1` in full. A fragment that
    /// matches one interface is still weaker evidence than a name that matches one interface, and
    /// the tiers have to be ordered by how much the device actually told us.
    #[test]
    fn an_exact_name_in_the_description_outranks_a_matching_id_fragment() {
        use super::remap_lldp_local_ports;
        let if_entries = dell_os10_if_entries();
        let mut loc_ports = HashMap::new();
        loc_ports.insert(4, loc_port_named(7, "4", "mgmt1/1/1"));

        let mut neighbors = vec![lldp_neighbor(4, "peer")];
        let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        assert_eq!(outcome, super::LocalPortOutcome::default());
        assert_eq!(
            neighbors[0].local_port_index, 1,
            "\"4\" ends at a boundary in ethernet1/1/4, but the device named mgmt1/1/1 outright"
        );
    }

    /// `interfaceIndex(2)` says the port id *is* an ifIndex, and the tier used to return whatever
    /// integer arrived without asking whether the device has an interface by that number. On a
    /// switch numbering its LLDP ports past 568 against 23 interfaces that is not a near miss:
    /// the neighbour reaches no interface, `convert_snmp_if_entry` discards it whole, and the
    /// switch reads as having no LLDP at all. An index naming nothing is not an answer, so the
    /// later tiers get their turn.
    #[test]
    fn an_advertised_index_naming_no_interface_falls_through() {
        use super::remap_lldp_local_ports;
        let if_entries = dell_os10_if_entries();
        let mut loc_ports = HashMap::new();
        loc_ports.insert(568, loc_port_named(2, "568", "ethernet1/1/14:1"));

        let mut neighbors = vec![lldp_neighbor(568, "EVILCORP")];
        let outcome = remap_lldp_local_ports(&mut neighbors, &loc_ports, &if_entries);

        assert_eq!(outcome, super::LocalPortOutcome::default());
        assert_eq!(
            neighbors[0].local_port_index, 15,
            "no interface has ifIndex 568, so the description has to place the neighbour"
        );
    }
}
