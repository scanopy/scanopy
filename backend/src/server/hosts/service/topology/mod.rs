//! LLDP and FDB link resolution.
use super::*;

use crate::daemon::discovery::types::warnings::{
    DiscoveryWarning, UnmatchedNeighbour, UnresolvedPort,
};

/// Why a far end could not be placed, carried alongside the identifiers that were tried.
///
/// The distinction is the whole point of naming these at all: `NotFound` is a gap in what has been
/// scanned and the operator can close it, `Ambiguous` is a device that *is* scanned but reports one
/// identifier for many ports, and `NoStrategy` is a gap on our side. Reporting all three as "did
/// not resolve" is what made a device-level edge un-triagable without a customer snmpwalk (GH #668).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnresolvedReason {
    NoStrategy,
    NotFound,
    Ambiguous,
}

impl UnresolvedReason {
    /// `None` for a resolution that succeeded — there is nothing to report.
    fn from_resolution(resolution: IdentityResolution) -> Option<Self> {
        match resolution {
            IdentityResolution::Resolved(_) => None,
            IdentityResolution::NoStrategy => Some(Self::NoStrategy),
            IdentityResolution::NotFound => Some(Self::NotFound),
            IdentityResolution::Ambiguous => Some(Self::Ambiguous),
        }
    }
}

/// The warning for a far end no strategy could place, or `None` when it is not worth reporting.
///
/// `NoStrategy` is deliberately silent on the host side: it counts the `cdp_address`-only rows the
/// resolver itself calls "nothing here to resolve", and a warning per one of those would bury the
/// two that mean something.
///
/// What each warning carries is what it takes to decide whether an unresolved neighbour is a
/// device that should have been scanned or one that never will be: which of our devices saw it, on
/// which port, and the identifier the far end advertised. That evidence used to exist only in a
/// log line, where the operator who needed it could not read it.
///
/// GH #701: takes the candidate's own [`InterfaceNeighborEvidence`] rather than reading it off
/// `interface` — one warning per candidate now, not per interface.
fn unmatched_neighbour_warning(
    interface: &Interface,
    evidence: &InterfaceNeighborEvidence,
    identifier: String,
    sys_name: Option<String>,
    reason: UnresolvedReason,
) -> Option<DiscoveryWarning> {
    let detail = UnmatchedNeighbour {
        host_id: interface.base.host_id,
        if_descr: interface.base.if_descr.clone().unwrap_or_default(),
        identifier,
        sys_name,
        address: evidence
            .advertised_identity()
            .address
            .map(|addr| addr.to_string()),
    };
    match reason {
        UnresolvedReason::NotFound => Some(DiscoveryWarning::LldpNeighbourNotFound(detail)),
        UnresolvedReason::Ambiguous => Some(DiscoveryWarning::LldpNeighbourAmbiguous(detail)),
        UnresolvedReason::NoStrategy => None,
    }
}

/// The far end behind a `NotFound` warning.
///
/// Derived at exactly the sites that warn, so what an operator is told and what is minted can never
/// be two different populations. `Ambiguous` is excluded on purpose: it means the identifier names
/// several hosts this network already holds, so the problem is duplicate records, not a missing
/// device.
///
/// An address is *not* required. Most far ends publish none — a chassis id and nothing else is the
/// common shape — and one is still a device nothing has contacted, which is the whole of what
/// `EntitySource::Inferred` claims. An address only decides whether the minted host can be placed
/// in a subnet, and that question belongs to range inference rather than to this gate.
fn unplaced_far_end(
    interface: &Interface,
    evidence: &InterfaceNeighborEvidence,
    reason: UnresolvedReason,
) -> Option<UnplacedFarEnd> {
    if reason != UnresolvedReason::NotFound {
        return None;
    }
    // The chassis id for LLDP, the device id for CDP. Both are what the far end calls itself, and
    // both are stored the way a scanned device's own identity is, so either can be the hinge the
    // minted host later merges on.
    let chassis_id = evidence
        .lldp_chassis_id
        .as_ref()
        .map(|id| id.identifier())
        .or_else(|| evidence.cdp_device_id.clone())
        .filter(|id| !id.trim().is_empty())?;

    let far_end_port = evidence.advertised_far_end_port();
    Some(UnplacedFarEnd {
        host_id: interface.base.host_id,
        if_descr: interface.base.if_descr.clone().unwrap_or_default(),
        sys_name: evidence
            .lldp_sys_name
            .clone()
            .or_else(|| evidence.cdp_device_id.clone()),
        chassis_id,
        address: evidence.advertised_identity().address,
        port_name: far_end_port.name.map(str::to_string),
        port_mac: far_end_port.mac.map(str::to_string),
        vlan_id: interface.base.native_vlan_id,
    })
}

/// The warning for a neighbour whose far-end *device* is known but whose port is not.
///
/// This is the row that draws a device-level edge instead of a port-to-port one — the "attached to
/// the whole switch" outcome, and the harder case to reason about from a counter alone: both
/// devices are on the map and the operator has nothing left to scan. All three reasons are
/// reported, because each names a different fix.
fn unresolved_port_warning(
    interface: &Interface,
    evidence: &InterfaceNeighborEvidence,
    remote_host_id: Uuid,
    port_id: Option<String>,
    reason: UnresolvedReason,
) -> DiscoveryWarning {
    let detail = UnresolvedPort {
        host_id: interface.base.host_id,
        if_descr: interface.base.if_descr.clone().unwrap_or_default(),
        remote_host_id,
        port_id,
        // `lldpRemPortDesc`, the last-resort tier. Carried because "the id failed and the
        // description was empty" and "both were tried and neither matched" call for different
        // fixes.
        port_desc: evidence.lldp_port_desc.clone(),
    };
    match reason {
        UnresolvedReason::NoStrategy => DiscoveryWarning::LldpPortNoStrategy(detail),
        UnresolvedReason::NotFound => DiscoveryWarning::LldpPortNotFound(detail),
        UnresolvedReason::Ambiguous => DiscoveryWarning::LldpPortAmbiguous(detail),
    }
}

/// Who names whom across a network's LLDP/CDP adjacencies, and the port pairs that follow from it.
///
/// Built once per resolution pass from *every* candidate in the network — not just the unresolved
/// ones. The count of ports joining two devices is the whole basis of the reciprocal tier, and
/// counting only the unresolved half would pair one leg of a LAG whose other leg happened to
/// resolve, which is exactly the arbitrary-port outcome the shared-MAC guard exists to prevent.
struct NeighborAdjacency {
    /// Every interface in the network with at least one live candidate, in whatever resolution
    /// state.
    interfaces: Vec<Interface>,
    /// Each interface's own candidates, loaded once here and reused by the resolution pass so it
    /// is not re-queried per interface.
    candidates: HashMap<Uuid, Vec<InterfaceNeighborCandidate>>,
    /// The far-end *device* verdict per candidate (keyed on the candidate's own id), computed once
    /// here and reused by the resolution pass so the chassis ladder is not run twice for the same
    /// row.
    host_of: HashMap<Uuid, IdentityResolution>,
    /// `(local_interface_id, remote_host_id) -> remote_interface_id`, for pairs where each side
    /// names the other on exactly one port. See `reciprocal.rs`'s module docs for why this is keyed
    /// on the pair rather than on the local interface alone.
    reciprocal: HashMap<(Uuid, Uuid), Uuid>,
}

/// What one resolution pass concluded.
struct NeighbourPass {
    stats: LldpResolutionStats,
    warnings: Vec<DiscoveryWarning>,
    /// Far ends that published an address and still matched nothing — the evidence the inference
    /// runs on, and empty on the second pass for anything the first one caused to be minted.
    unplaced: Vec<UnplacedFarEnd>,
}

mod inference;
mod reciprocal;

use crate::server::interface_neighbors::r#impl::base::{
    InterfaceNeighborCandidate, InterfaceNeighborEvidence, Neighbor,
};
use crate::server::interfaces::r#impl::base::InterfaceBase;
use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
use crate::server::subnets::r#impl::inference::UnplacedFarEnd;

use reciprocal::PortBinding;

impl HostService {
    // =========================================================================
    // LLDP link resolution
    // =========================================================================

    /// Resolve LLDP links for all interfaces in a network.
    ///
    /// Called by DiscoveryService when a discovery session completes successfully.
    /// This resolves LLDP neighbor data (chassis ID, port ID) to actual database
    /// entity references via the Neighbor enum.
    ///
    /// Resolution states:
    /// - Full resolution: Both host and port identified → `Neighbor::Interface(id)`
    /// - Partial resolution: Only host identified → `Neighbor::Host(id)`
    ///
    /// Returns the statistics and the operator-facing summary of what could not be placed.
    /// Resolve every neighbour once, and report the far ends nothing could place.
    ///
    /// Split out from [`Self::resolve_lldp_links`] so it can be run a second time after far-end
    /// subnets and hosts have been minted: those hosts are resolvable the moment they exist, and
    /// re-running here is the difference between a link appearing now and appearing after the next
    /// scan.
    ///
    /// GH #701: a port can carry several candidates now, so this loops per candidate (grouped by
    /// interface where interface-level context — existing bindings, `fdb_macs`, host id — is
    /// needed) and writes the whole desired adjacency set for an interface in one
    /// `reconcile_interface_neighbors` call, rather than one `interface.base.neighbor` field write.
    async fn resolve_neighbours_once(&self, network_id: Uuid) -> Result<NeighbourPass> {
        let resolver = self.lldp_inventory_snapshot(network_id).await?;

        // The instant before which neighbour evidence counts as stale, from the network's own
        // window — the same helper the `?stale=` list filter uses, so a link and a host cannot
        // disagree about what stale means. A network that no longer exists yields no cutoff, and
        // the fallback makes every row read current: an orphaned FK must never tear bindings down.
        let evidence_cutoff = self
            .network_service
            .stale_cutoffs(&[network_id])
            .await?
            .into_iter()
            .find(|(id, _)| *id == network_id)
            .map(|(_, cutoff)| cutoff)
            .unwrap_or(DateTime::<Utc>::MIN_UTC);

        // Who names whom, and which of those pairs are unambiguous in both directions. Computed
        // before anything is written, because it is the authority both for the reciprocal tier
        // below and for deciding whether an existing MAC-matched binding still stands.
        let adjacency = self
            .build_neighbor_adjacency(network_id, &resolver, evidence_cutoff)
            .await?;
        let reciprocal = adjacency.reciprocal;
        let host_of = adjacency.host_of;
        let candidates_by_interface = adjacency.candidates;
        let interfaces = adjacency.interfaces;

        // Existing resolved rows, batched once for the whole pass rather than per interface.
        let interface_ids: Vec<Uuid> = interfaces.iter().map(|i| i.id).collect();
        let mut existing_by_interface = self
            .interface_neighbor_service
            .resolved_for_interfaces(&interface_ids)
            .await?;

        // The host behind every already-bound far-end port, in one query rather than one each —
        // needed to re-key an existing full-resolution row's bound interface back to the remote
        // host `reciprocal`/`re_examine_port_binding` key on.
        let bound_ids: Vec<Uuid> = existing_by_interface
            .values()
            .flatten()
            .filter_map(|row| row.neighbor.interface_id())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let mut host_of_interface: HashMap<Uuid, Uuid> = HashMap::new();
        if !bound_ids.is_empty() {
            let filter = StorableFilter::<Interface>::new_from_entity_ids(&bound_ids).live();
            for bound in self.interface_service.get_all(filter).await? {
                host_of_interface.insert(bound.id, bound.base.host_id);
            }
        }

        let mut stats = LldpResolutionStats::default();
        let mut warnings: Vec<DiscoveryWarning> = Vec::new();
        // Far ends that told us where they live and still matched nothing. Pooled across the whole
        // network rather than per device: two switches naming far ends in one range must produce
        // one subnet, and only the server sees both.
        let mut unplaced: Vec<UnplacedFarEnd> = Vec::new();
        // (host, advertised port name, advertised port MAC) for far ends resolved to a device but
        // to no port of it. Collected here and acted on after the loop rather than mid-tier, so the
        // resolution pass stays a read of identities and a write of neighbours.
        let mut advertised_ports: Vec<(Uuid, Option<String>, Option<String>)> = Vec::new();
        let mut reopened = 0usize;
        let mut rebound = 0usize;
        let scan_time = Utc::now();

        for interface in &interfaces {
            let candidates = candidates_by_interface
                .get(&interface.id)
                .cloned()
                .unwrap_or_default();
            let existing_rows = existing_by_interface
                .remove(&interface.id)
                .unwrap_or_default();

            // Seed with every currently-stored adjacency, keyed by remote host — "stale beats
            // deleted": a remote host this pass gets no chance to re-confirm (no live candidate
            // names it any more) is carried forward unchanged rather than dropped, the same
            // direction `create.rs`'s interface prune takes when a scan is incomplete.
            let mut desired: HashMap<Uuid, (Neighbor, Option<DateTime<Utc>>)> = HashMap::new();
            for row in &existing_rows {
                let remote_host_id = match row.neighbor {
                    Neighbor::Interface(id) => host_of_interface.get(&id).copied(),
                    Neighbor::Host(id) => Some(id),
                };
                if let Some(remote_host_id) = remote_host_id {
                    desired.insert(remote_host_id, (row.neighbor, row.neighbor_seen_at));
                }
            }

            // A row that already names a port is re-examined rather than resolved: it is not a
            // link that failed, it is one that may have been placed on a MAC the far end repeats
            // across every port, which looks authoritative and is not.
            for row in existing_rows
                .iter()
                .filter(|r| r.neighbor.is_full_resolution())
            {
                let bound_id = row.neighbor.interface_id().expect("full resolution");
                let Some(&remote_host_id) = host_of_interface.get(&bound_id) else {
                    continue;
                };
                match self
                    .re_examine_port_binding(
                        interface,
                        bound_id,
                        remote_host_id,
                        &candidates,
                        &reciprocal,
                        &resolver,
                    )
                    .await?
                {
                    PortBinding::Stands => {}
                    PortBinding::Rebind(paired) => {
                        rebound += 1;
                        desired.insert(
                            remote_host_id,
                            (Neighbor::Interface(paired), row.neighbor_seen_at),
                        );
                    }
                    // Downgraded to the far-end device, which was never in doubt, and then run
                    // through the tiers below in this same pass so a tier that *can* name a port
                    // gets its turn immediately rather than a scan later.
                    PortBinding::Reopen(remote_host_id) => {
                        reopened += 1;
                        desired.insert(
                            remote_host_id,
                            (Neighbor::Host(remote_host_id), row.neighbor_seen_at),
                        );
                    }
                }
            }

            // Resolve every candidate independently. Two candidates naming the same remote host
            // (an LLDP entry and a CDP entry for one device) both contribute to the *same*
            // adjacency — `desired` is keyed by remote host, so the second one to resolve wins the
            // slot, which is fine: they describe one link, not two.
            for candidate in &candidates {
                let evidence = &candidate.base.evidence;
                if evidence.lldp_chassis_id.is_none()
                    && evidence.cdp_device_id.is_none()
                    && evidence.cdp_address.is_none()
                {
                    continue;
                }

                stats.total += 1;

                let resolved_neighbor = if let Some(ref chassis_id) = evidence.lldp_chassis_id {
                    let host = host_of
                        .get(&candidate.id)
                        .copied()
                        .unwrap_or(IdentityResolution::NoStrategy);
                    if let Some(reason) = UnresolvedReason::from_resolution(host) {
                        warnings.extend(unmatched_neighbour_warning(
                            interface,
                            evidence,
                            chassis_id.identifier(),
                            evidence.lldp_sys_name.clone(),
                            reason,
                        ));
                        unplaced.extend(unplaced_far_end(interface, evidence, reason));
                    }
                    match stats.record_host(host) {
                        None => None,
                        Some(host_id) => {
                            let port = match evidence.lldp_port_id {
                                Some(ref port_id) => {
                                    port_id.resolve_if_entry_id(&resolver, host_id).await
                                }
                                None => IdentityResolution::NoStrategy,
                            };
                            // Last resort: the port *description*.
                            let port = match port {
                                IdentityResolution::Resolved(id) => {
                                    IdentityResolution::Resolved(id)
                                }
                                unresolved => match evidence.lldp_port_desc.as_deref() {
                                    Some(desc) if !desc.trim().is_empty() => {
                                        match resolver.find_if_entry_by_name(desc, host_id).await {
                                            Some(id) => IdentityResolution::Resolved(id),
                                            None => unresolved,
                                        }
                                    }
                                    _ => unresolved,
                                },
                            };
                            let port = match HostService::pair_reciprocally(
                                port,
                                interface.id,
                                host_id,
                                &reciprocal,
                            ) {
                                Some(paired) => {
                                    stats.ports_resolved_reciprocal += 1;
                                    paired
                                }
                                None => port,
                            };
                            if let Some(reason) = UnresolvedReason::from_resolution(port) {
                                warnings.push(unresolved_port_warning(
                                    interface,
                                    evidence,
                                    host_id,
                                    evidence.lldp_port_id.as_ref().map(|p| format!("{p:?}")),
                                    reason,
                                ));
                            }
                            Some((host_id, stats.record_port(port, host_id)))
                        }
                    }
                } else if let Some(ref device_id) = evidence.cdp_device_id {
                    let host = host_of
                        .get(&candidate.id)
                        .copied()
                        .unwrap_or(IdentityResolution::NoStrategy);
                    if let Some(reason) = UnresolvedReason::from_resolution(host) {
                        warnings.extend(unmatched_neighbour_warning(
                            interface,
                            evidence,
                            device_id.clone(),
                            None,
                            reason,
                        ));
                        unplaced.extend(unplaced_far_end(interface, evidence, reason));
                    }
                    match stats.record_host(host) {
                        None => None,
                        Some(host_id) => {
                            let port = match evidence.cdp_port_id {
                                Some(ref port_id) => IdentityResolution::found(
                                    resolver.find_if_entry_by_name(port_id, host_id).await,
                                ),
                                None => IdentityResolution::NoStrategy,
                            };
                            let port = match HostService::pair_reciprocally(
                                port,
                                interface.id,
                                host_id,
                                &reciprocal,
                            ) {
                                Some(paired) => {
                                    stats.ports_resolved_reciprocal += 1;
                                    paired
                                }
                                None => port,
                            };
                            if let Some(reason) = UnresolvedReason::from_resolution(port) {
                                warnings.push(unresolved_port_warning(
                                    interface,
                                    evidence,
                                    host_id,
                                    evidence
                                        .cdp_port_id
                                        .as_ref()
                                        .map(|id| format!("CdpPortId({id:?})")),
                                    reason,
                                ));
                            }
                            Some((host_id, stats.record_port(port, host_id)))
                        }
                    }
                } else {
                    // Admitted by the loop only because `cdp_address` is set. It is a management
                    // address, so it names no *port* — but it does name a device.
                    let host = host_of
                        .get(&candidate.id)
                        .copied()
                        .unwrap_or(IdentityResolution::NoStrategy);
                    if let Some(reason) = UnresolvedReason::from_resolution(host) {
                        warnings.extend(unmatched_neighbour_warning(
                            interface,
                            evidence,
                            evidence
                                .cdp_address
                                .map(|addr| addr.to_string())
                                .unwrap_or_default(),
                            None,
                            reason,
                        ));
                        unplaced.extend(unplaced_far_end(interface, evidence, reason));
                    }
                    stats
                        .record_host(host)
                        .map(|host_id| (host_id, Neighbor::Host(host_id)))
                };

                if let Some((host_id, neighbor)) = resolved_neighbor {
                    // Evidence freshness for this adjacency = when this candidate's evidence was
                    // last confirmed by a scan (its `created_at` — see
                    // `InterfaceNeighborService::replace_candidates_from_discovery`).
                    desired.insert(host_id, (neighbor, Some(candidate.created_at)));

                    // Resolved to a device but to no port of it. The advertisement still names
                    // that port, and for a device nothing can walk that is the only description
                    // of it there will ever be.
                    if let Neighbor::Host(host_id) = neighbor {
                        let port = evidence.advertised_far_end_port();
                        if port.name.is_some() || port.mac.is_some() {
                            advertised_ports.push((
                                host_id,
                                port.name.map(str::to_string),
                                port.mac.map(str::to_string),
                            ));
                        }
                    }
                }
            }

            let final_desired: Vec<(Neighbor, Option<DateTime<Utc>>)> =
                desired.into_values().collect();
            self.interface_neighbor_service
                .reconcile_interface_neighbors(
                    network_id,
                    interface.id,
                    &final_desired,
                    scan_time,
                    None,
                )
                .await?;
        }

        self.record_advertised_far_end_ports(network_id, advertised_ports)
            .await;

        tracing::info!(
            network_id = %network_id,
            total = stats.total,
            hosts_resolved = stats.hosts_resolved,
            ports_resolved = stats.ports_resolved,
            ports_resolved_reciprocal = stats.ports_resolved_reciprocal,
            // The five per-reason failure counters this line used to carry are now exactly the
            // count of their `DiscoveryWarning`s, and two sources for one number can only
            // disagree. `host_no_strategy` stays because nothing warns on it: it counts the
            // `cdp_address`-only rows there was never anything to resolve in.
            host_no_strategy = stats.host_no_strategy,
            reopened,
            rebound,
            "LLDP/CDP link resolution complete"
        );

        Ok(NeighbourPass {
            stats,
            warnings,
            unplaced,
        })
    }

    /// Resolve LLDP links for all interfaces in a network, inferring what is missing.
    ///
    /// Two passes at most. The first resolves what it can and collects the far ends that told us
    /// where they live and still matched nothing; those become subnets and hosts; the second pass
    /// then places the neighbours naming them.
    ///
    /// The second pass's findings *replace* the first's rather than adding to them. A far end that
    /// resolves once its host exists is no longer an unmatched neighbour, and reporting both would
    /// tell an operator that the same devices are missing and were just added.
    pub async fn resolve_lldp_links(
        &self,
        network_id: Uuid,
        scan_time: DateTime<Utc>,
    ) -> Result<LldpResolutionOutcome> {
        let first = self.resolve_neighbours_once(network_id).await?;

        // The plan's host limit, built the way the daemon batch path builds it
        // (`DaemonService::process_discovery_entities`). Minting runs outside both existing gates,
        // so without this it would quietly outrun the limit while still counting towards the number
        // a customer is shown on their dashboard and in their usage email.
        let limit_ctx = self.host_limit_context(network_id).await;

        let inferred = self
            .infer_far_end_subnets(network_id, first.unplaced, limit_ctx.as_ref(), scan_time)
            .await?;

        // A pass that created nothing has nothing new to resolve against, whatever the standing
        // report says about ranges awaiting confirmation.
        if inferred.minted_host_ids.is_empty() {
            let mut warnings = first.warnings;
            warnings.extend(inferred.warnings);
            return Ok(LldpResolutionOutcome {
                stats: first.stats,
                warnings,
                minted_host_ids: Vec::new(),
            });
        }

        // Exactly one re-run, never a loop: the second pass mints nothing, so a far end it still
        // cannot place is one no further pass would place either.
        let second = self.resolve_neighbours_once(network_id).await?;
        let mut warnings = second.warnings;
        warnings.extend(inferred.warnings);

        Ok(LldpResolutionOutcome {
            stats: second.stats,
            warnings,
            minted_host_ids: inferred.minted_host_ids,
        })
    }

    /// Give a far end the port it advertised, where nothing else has ever described one.
    ///
    /// A device that answers only its system MIB has no ifTable to read, so it draws in L2 as a
    /// container with nothing in it — a box with a name and no ports, which reads as a rendering
    /// fault rather than as "this device tells us nothing about itself". The neighbour that named
    /// it *did* say which port it answers on, and for such a device that is the only description of
    /// that port there will ever be.
    ///
    /// **Only where the host has no interfaces at all.** With nothing stored there is nothing to
    /// duplicate, which is the whole risk: a device that advertises `1` while its own ifTable calls
    /// the port `Gi0/1` would otherwise gain a phantom beside the real row. Should such a device
    /// later be walked properly, the authoritative-walk prune in `create_with_children` removes
    /// anything the walk did not report, so even that case heals itself.
    ///
    /// Distinct from the minting path, which builds these for a host that did not exist. Here the
    /// host is real and already resolved; only its port is missing.
    async fn record_advertised_far_end_ports(
        &self,
        network_id: Uuid,
        advertised: Vec<(Uuid, Option<String>, Option<String>)>,
    ) {
        let mut seen: HashSet<Uuid> = HashSet::new();
        for (host_id, name, mac) in advertised {
            if !seen.insert(host_id) {
                continue;
            }

            let existing = self
                .interface_service
                .get_all(StorableFilter::<Interface>::new_from_host_ids(&[host_id]).live())
                .await;
            match existing {
                Ok(rows) if rows.is_empty() => {}
                // Anything already stored describes this device better than an advertisement does.
                Ok(_) => continue,
                Err(e) => {
                    tracing::warn!(
                        host_id = %host_id,
                        error = %e,
                        "Could not tell whether a far end already has ports; leaving it alone"
                    );
                    continue;
                }
            }

            let mac_address = mac.as_deref().and_then(|m| m.parse::<MacAddress>().ok());
            let descr = match (&name, &mac_address) {
                (Some(name), _) => name.clone(),
                (None, Some(mac)) => mac.to_string(),
                (None, None) => continue,
            };

            let interface = Interface::new(InterfaceBase {
                network_id,
                host_id,
                if_descr: Some(descr),
                if_name: name,
                // The port id a neighbour advertised for itself. Announced on a link anything
                // could have spoken on, not something we asked the far end for.
                mac_address: mac_address
                    .map(|m| MacEvidence::new(MacEvidenceValue(m), AttributeSource::LldpChassisId)),
                ..Default::default()
            });

            match self
                .interface_service
                .create(interface, AuthenticatedEntity::System)
                .await
            {
                Ok(created) => tracing::info!(
                    host_id = %host_id,
                    port = %created.base.if_descr.as_deref().unwrap_or("?"),
                    "Recorded the port a neighbour named for a device that describes none itself"
                ),
                Err(e) => tracing::warn!(
                    host_id = %host_id,
                    error = %e,
                    "Could not record the port a neighbour named for a far end"
                ),
            }
        }
    }

    /// Read this network's identity columns once, for the pass to resolve against.
    ///
    /// The pass asks the same few questions per neighbour-bearing interface, and answering each
    /// with its own query made it scale with round-trips: ~330 ms on 145 interfaces, and the
    /// completion request is what waits for it. Three loads replace thousands of round-trips.
    ///
    /// Safe to hold across the whole pass because every lookup keys on an identity column, and
    /// resolution now writes to `interface_neighbor_interfaces`/`interface_neighbor_hosts` rather
    /// than to any column this snapshot reads.
    async fn lldp_inventory_snapshot(&self, network_id: Uuid) -> Result<LldpInventorySnapshot> {
        let network = [network_id];
        let hosts = self
            .get_all(StorableFilter::<Host>::new_from_network_ids(&network).live())
            .await?;
        let interfaces = self
            .interface_service
            .get_all(StorableFilter::<Interface>::new_from_network_ids(&network).live())
            .await?;
        let addresses = self
            .ip_address_service
            .get_all(StorableFilter::<IPAddress>::new_from_network_ids(&network).live())
            .await?;

        Ok(LldpInventorySnapshot::new(&hosts, &interfaces, &addresses))
    }

    /// How many interfaces on this network advertise a neighbour.
    ///
    /// Only for the warning raised when resolution is cut short, which is why it costs a query
    /// rather than being threaded out of the pass: on every other completion the pass returns
    /// normally and nothing asks. Best-effort — a warning that cannot say "how many" is still
    /// worth raising, so a failure here reports zero rather than suppressing the warning.
    pub async fn neighbour_bearing_interface_count(&self, network_id: Uuid) -> u32 {
        self.interface_neighbor_service
            .candidates_for_network(network_id)
            .await
            .map(|candidates| {
                candidates
                    .iter()
                    .map(|c| c.base.interface_id)
                    .collect::<HashSet<_>>()
                    .len() as u32
            })
            .unwrap_or(0)
    }

    /// Interfaces with an unresolved single-MAC FDB entry — cheaper than `resolve_fdb_links`
    /// itself since it never fetches or hydrates a row. Only for the warning raised when
    /// resolution is cut short, the same reasoning as `neighbour_bearing_interface_count` above.
    pub async fn unresolved_fdb_interface_count(&self, network_id: Uuid) -> u32 {
        self.interface_service
            .storage()
            .count(StorableFilter::<Interface>::new_for_unresolved_fdb_in_network(network_id))
            .await
            .unwrap_or(0) as u32
    }

    /// Resolve FDB (bridge forwarding database) single-MAC ports to neighbor links.
    /// Called after resolve_lldp_links — only processes ports without LLDP/CDP data
    /// that have exactly one learned MAC address (direct physical connection).
    pub async fn resolve_fdb_links(
        &self,
        network_id: Uuid,
        scan_time: DateTime<Utc>,
    ) -> Result<u32> {
        let resolver = LldpResolverImpl::new(
            self.interface_service.clone(),
            self.ip_address_service.clone(),
            self.storage.clone(),
        );

        let filter = StorableFilter::<Interface>::new_for_unresolved_fdb_in_network(network_id);
        let unresolved = self.interface_service.get_all(filter).await?;

        let mut resolved_count: u32 = 0;

        for interface in unresolved {
            // The SQL filter above already restricts to exactly one learned MAC
            // (`jsonb_array_length(fdb_col) = 1`); this re-check is defense-in-depth against a
            // future filter regression, not a path a passing filter can reach.
            let mac = match &interface.base.fdb_macs {
                Some(macs) if macs.len() == 1 => &macs[0],
                _ => continue,
            };

            // Try to find host by MAC. A MAC on more than one device names none of them, so an
            // ambiguous verdict leaves the row unresolved rather than picking a side.
            let IdentityResolution::Resolved(host_id) =
                resolver.find_host_by_mac(mac, network_id).await
            else {
                continue;
            };

            // Try full resolution (specific port). A far end that repeats one MAC across its ports
            // names no single port, so the link stays at device level rather than being attached to
            // whichever port the database returned first.
            let neighbor = match resolver.find_if_entry_by_mac(mac, host_id).await {
                IdentityResolution::Resolved(interface_id) => Neighbor::Interface(interface_id),
                _ => Neighbor::Host(host_id),
            };

            self.interface_neighbor_service
                .reconcile_interface_neighbors(
                    network_id,
                    interface.id,
                    &[(neighbor, Some(scan_time))],
                    scan_time,
                    None,
                )
                .await?;
            resolved_count += 1;
        }

        if resolved_count > 0 {
            tracing::debug!(
                network_id = %network_id,
                resolved = resolved_count,
                "FDB link resolution complete"
            );
        }

        Ok(resolved_count)
    }
}
