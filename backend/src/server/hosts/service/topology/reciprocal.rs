//! The reciprocal-LLDP pairing tier.
//!
//! Every other tier identifies a far-end port from something the far end advertised about itself,
//! which fails outright on the switch families that report one chassis MAC across every port
//! (D-Link, TP-Link/Omada, UniFi, Westermo): the identifier names the device and nothing narrower.
//! This tier identifies nothing at all. It observes that two devices name each other, and binds the
//! two *local* interfaces that did the naming — both already attached by `ifIndex` on their own
//! device, so the far-end port never has to be recognised and the shared MAC cannot get in the way.
//!
//! GH #701: a port can now carry several candidates (`InterfaceNeighborCandidate`), so the
//! adjacency map keys below moved from "one slot per local interface" to "one slot per (local
//! interface, remote host)" — see `NeighborAdjacency` in `mod.rs` and the module docs there for the
//! confirmed root cause this re-keying fixes.
use super::*;

impl HostService {
    /// The reciprocal-LLDP tier: bind two interfaces that name each other and nothing else.
    ///
    /// Consulted only once the port-id and port-description tiers have failed, and only for a pair
    /// where **each side has exactly one** interface pointing at the other. Both endpoints are
    /// *locally* known interfaces, attached by `ifIndex` on their own device, so the far end's port
    /// never has to be identified from what it advertises — which is what makes this work on the
    /// switch families that report one chassis MAC across every port (D-Link, TP-Link/Omada,
    /// UniFi, Westermo) where no MAC can name a port.
    ///
    /// A LAG between two switches is genuinely ambiguous and is left device-level: guessing which
    /// member is which is the same arbitrary-port outcome the shared-MAC guard exists to prevent.
    ///
    /// Keyed on `(local_interface_id, remote_host_id)` rather than `local_interface_id` alone —
    /// the confirmed root cause of both "only 2 of 3 edges render" and "the stored neighbour flips
    /// between identical scans" (GH #701): once a port can name more than one remote host, a map
    /// keyed on the local interface alone has only one slot for however many distinct hosts that
    /// port names, and each `reciprocal.insert()` for a later host silently overwrote the earlier
    /// one. Including the remote host in the key gives every (port, remote device) pair its own
    /// slot, so the "does this pairing agree with the host the ladder settled on" check that used
    /// to be a separate `paired_host == host_id` comparison is now the lookup itself.
    pub(super) fn pair_reciprocally(
        port: IdentityResolution,
        interface_id: Uuid,
        host_id: Uuid,
        reciprocal: &HashMap<(Uuid, Uuid), Uuid>,
    ) -> Option<IdentityResolution> {
        if matches!(port, IdentityResolution::Resolved(_)) {
            return None;
        }
        reciprocal
            .get(&(interface_id, host_id))
            .map(|&paired_id| IdentityResolution::Resolved(paired_id))
    }

    /// Build the network's neighbour adjacency and the reciprocal pairs that follow from it.
    ///
    /// One query for every live candidate in the network, one query for the interfaces those
    /// candidates belong to, and the chassis/CDP ladder run fresh for every candidate — GH #701
    /// dropped the "reuse an already-bound port's stored verdict" shortcut the single-neighbour
    /// version had, since a bound interface no longer identifies which of several candidates
    /// produced it. The ladder runs against `resolver`'s in-memory snapshot, not a query per call,
    /// so this costs CPU, not round-trips.
    ///
    /// `evidence_cutoff` is the instant before which a candidate's `created_at` counts as stale —
    /// the network's own staleness window, so a link is judged by the same rule and the same
    /// setting as every other freshness verdict. Candidates are replaced wholesale every complete
    /// scan (see `InterfaceNeighborService::replace_candidates_from_discovery`), so a candidate's
    /// `created_at` is "last scan that confirmed this exact piece of evidence", not "last time this
    /// row was written to storage" — a group preserved because a walk was cut short keeps its
    /// original `created_at`, and so reads as stale exactly when it should.
    pub(super) async fn build_neighbor_adjacency(
        &self,
        network_id: Uuid,
        resolver: &impl LldpResolver,
        evidence_cutoff: DateTime<Utc>,
    ) -> Result<NeighborAdjacency> {
        let all_candidates = self
            .interface_neighbor_service
            .candidates_for_network(network_id)
            .await?;

        let mut candidates_by_interface: HashMap<Uuid, Vec<InterfaceNeighborCandidate>> =
            HashMap::new();
        for candidate in all_candidates {
            candidates_by_interface
                .entry(candidate.base.interface_id)
                .or_default()
                .push(candidate);
        }

        let interface_ids: Vec<Uuid> = candidates_by_interface.keys().copied().collect();
        let interfaces = if interface_ids.is_empty() {
            Vec::new()
        } else {
            let filter = StorableFilter::<Interface>::new_from_entity_ids(&interface_ids).live();
            self.interface_service.get_all(filter).await?
        };

        let mut host_of: HashMap<Uuid, IdentityResolution> = HashMap::new();
        // (local host, remote host) -> the local ports that name that remote host.
        let mut ports_between: HashMap<(Uuid, Uuid), Vec<NamingPort>> = HashMap::new();

        for interface in &interfaces {
            let candidates = candidates_by_interface
                .get(&interface.id)
                .map(Vec::as_slice)
                .unwrap_or_default();

            // Two candidates on this interface naming the same remote host (an LLDP entry and a
            // CDP entry for one device) must collapse into one `NamingPort` for that host, or the
            // "exactly one port each way" LAG guard below sees two ports where there is only one
            // and downgrades a still-precise link to device-level (GH #701's dedup rule).
            let mut named_this_interface: HashMap<Uuid, usize> = HashMap::new();

            for candidate in candidates {
                let evidence = &candidate.base.evidence;
                let resolution = if let Some(ref chassis_id) = evidence.lldp_chassis_id {
                    chassis_id
                        .resolve_host_id(resolver, network_id, evidence.advertised_identity())
                        .await
                } else if evidence.cdp_device_id.is_some() || evidence.cdp_address.is_some() {
                    // CDP carries no chassis id, so there is no subtype tier to run — but the
                    // device id is a `sysName` and `cdpCacheAddress` is a management address,
                    // which are exactly the two tiers above. Running them here is what stops a
                    // neighbour from being unresolvable purely for arriving over CDP.
                    evidence
                        .advertised_identity()
                        .resolve_host_id(resolver, network_id)
                        .await
                } else {
                    IdentityResolution::NoStrategy
                };
                host_of.insert(candidate.id, resolution);

                // `None` is unknown, not stale — a candidate the resolver could not date (should
                // not happen; every candidate has a `created_at`) must not lose its binding.
                let evidence_current = candidate.created_at >= evidence_cutoff;

                // A device naming itself contributes no adjacency and must never pair.
                if let IdentityResolution::Resolved(remote_host_id) = resolution
                    && remote_host_id != interface.base.host_id
                {
                    let bucket = ports_between
                        .entry((interface.base.host_id, remote_host_id))
                        .or_default();
                    match named_this_interface.get(&remote_host_id) {
                        Some(&idx) => {
                            // Same interface, same remote host, another candidate: OR the
                            // freshness rather than adding a second `NamingPort`.
                            bucket[idx].evidence_current |= evidence_current;
                        }
                        None => {
                            named_this_interface.insert(remote_host_id, bucket.len());
                            bucket.push(NamingPort {
                                interface_id: interface.id,
                                evidence_current,
                            });
                        }
                    }
                }
            }
        }

        Ok(NeighborAdjacency {
            interfaces,
            candidates: candidates_by_interface,
            host_of,
            reciprocal: reciprocal_pairs(&ports_between),
        })
    }

    /// Whether an existing resolved-interface binding still stands, and what to do if it does not.
    ///
    /// Runs once per live `interface_neighbor_interfaces` row rather than once per local
    /// interface (GH #701: a port can have several such rows now). `remote_host_id` is the host
    /// behind `bound_id` — the caller already has it from `NeighborAdjacency`'s interface list, so
    /// this does not re-query for it.
    ///
    /// Reciprocal evidence is consulted first, and that is what keeps this from churning: a binding
    /// the pairing confirms is left untouched rather than being torn down and rebuilt on every
    /// scan, which at the customer's scale would be an SCD2-adjacent write per link per scan.
    pub(super) async fn re_examine_port_binding(
        &self,
        interface: &Interface,
        bound_id: Uuid,
        remote_host_id: Uuid,
        candidates: &[InterfaceNeighborCandidate],
        reciprocal: &HashMap<(Uuid, Uuid), Uuid>,
        resolver: &impl LldpResolver,
    ) -> Result<PortBinding> {
        if let Some(&paired_id) = reciprocal.get(&(interface.id, remote_host_id)) {
            return Ok(if paired_id == bound_id {
                PortBinding::Stands
            } else {
                // Two locally-known interfaces naming each other beat an identifier the far end
                // advertised about itself, which is what the current binding rests on.
                PortBinding::Rebind(paired_id)
            });
        }

        // Only the tiers that match on a MAC rest on that MAC's uniqueness; a port matched on a
        // name, an ifIndex or an IP is unaffected and re-opening it would tear down a healthy
        // link. `candidates` no longer carries a direct pointer to which one placed this specific
        // binding (resolved rows aren't FK'd to a candidate — see `interface_neighbors`'s module
        // docs), so this checks whether *any* current candidate rests on a MAC, falling back to
        // the FDB single-MAC condition when this interface has no LLDP/CDP evidence at all — the
        // same disjunction `Interface::port_bound_by_mac` used to test in one place, now composed
        // from its two successors.
        let mac_bound = candidates
            .iter()
            .any(|c| c.base.evidence.port_bound_by_mac())
            || (candidates
                .iter()
                .all(|c| !c.base.evidence.has_lldp_data() && !c.base.evidence.has_cdp_data())
                && interface
                    .base
                    .fdb_macs
                    .as_ref()
                    .is_some_and(|macs| macs.len() == 1));
        if !mac_bound {
            return Ok(PortBinding::Stands);
        }

        let bound_filter = StorableFilter::<Interface>::new_from_entity_ids(&[bound_id]).live();
        let Some(bound) = self
            .interface_service
            .get_unique(bound_filter)
            .await?
            .at_most_one()?
        else {
            return Ok(PortBinding::Stands);
        };
        let Some(mac) = bound.base.mac_address else {
            return Ok(PortBinding::Stands);
        };

        // Ask the guard itself rather than re-deriving "is this MAC unique on that device", so
        // the two can never disagree about which bindings are legitimate.
        if matches!(
            resolver
                .find_if_entry_by_mac(&mac.to_string(), bound.base.host_id)
                .await,
            IdentityResolution::Ambiguous
        ) {
            Ok(PortBinding::Reopen(remote_host_id))
        } else {
            Ok(PortBinding::Stands)
        }
    }
}

/// A local port that names a remote host, and whether it still says so from evidence a scan
/// actually carried — as opposed to from a stored binding already pointing at the far end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NamingPort {
    interface_id: Uuid,
    evidence_current: bool,
}

/// The pairs where each device names the other on exactly one port.
///
/// The "exactly one each way" rule is the whole guard. Two switches joined by a LAG name each other
/// on several ports and there is no evidence in LLDP for which member faces which — pairing them
/// would name an arbitrary port and draw it as authoritative, the precise failure the shared-MAC
/// guard exists to prevent. Those stay device-level.
///
/// The tally counts *every* naming port, stored bindings included — that is what keeps a LAG whose
/// members have already resolved device-level, and it must not be narrowed. Only the emission is
/// gated on evidence, below.
///
/// GH #701: returns `HashMap<(local_interface_id, remote_host_id), remote_interface_id>` instead
/// of `HashMap<local_interface_id, (remote_interface_id, remote_host_id)>` — the confirmed root
/// cause fix. A local interface can now name several distinct remote hosts, each pairing
/// independently; the old key gave every one of them the same single slot, and the last write won.
fn reciprocal_pairs(
    ports_between: &HashMap<(Uuid, Uuid), Vec<NamingPort>>,
) -> HashMap<(Uuid, Uuid), Uuid> {
    let mut reciprocal = HashMap::new();
    for ((local_host, remote_host), local_ports) in ports_between {
        let [local_port] = local_ports[..] else {
            continue;
        };
        let Some(remote_ports) = ports_between.get(&(*remote_host, *local_host)) else {
            continue;
        };
        let [remote_port] = remote_ports[..] else {
            continue;
        };
        // A pair is two devices *naming* each other. A stored binding read back as adjacency is
        // the binding under examination, so a pair resting on one confirms only itself — and
        // `re_examine_port_binding` would then call a link whose evidence has completely
        // disappeared confirmed, for ever. Both sides are required: the far end may well be
        // answering perfectly while it is the local port whose evidence vanished, and gating one
        // side alone would downgrade one endpoint while the other kept pointing at it.
        if !local_port.evidence_current || !remote_port.evidence_current {
            continue;
        }
        reciprocal.insert(
            (local_port.interface_id, *remote_host),
            remote_port.interface_id,
        );
    }
    reciprocal
}

/// What re-examining an existing port binding concluded.
pub(super) enum PortBinding {
    /// Leave it alone — no write.
    Stands,
    /// Reciprocal evidence names a different port on the same device.
    Rebind(Uuid),
    /// The binding rests on a MAC the far end repeats across its ports. Downgraded to the far-end
    /// device (which was never in doubt) and retried by the tiers in the same pass.
    Reopen(Uuid),
}

/// The reciprocal tier's decision rule, exercised without a database.
///
/// What is under test is which adjacency shapes are allowed to produce a port-precise link — the
/// question the shared-MAC guard turns on — not the SQL that assembles them.
#[cfg(test)]
mod reciprocal_tests {
    use super::*;

    /// A port naming a neighbour from evidence a scan actually carried.
    fn evidenced(interface_id: Uuid) -> NamingPort {
        NamingPort {
            interface_id,
            evidence_current: true,
        }
    }

    /// A port that names a neighbour only because a stored binding still points at it — the shape
    /// a link takes once its LLDP/CDP/FDB evidence has stopped arriving.
    fn unevidenced(interface_id: Uuid) -> NamingPort {
        NamingPort {
            interface_id,
            evidence_current: false,
        }
    }

    /// Two switches, one cable, each naming the other on one port. Neither can identify the
    /// other's port from what it advertises (both report one chassis MAC across every port), but
    /// both endpoints are locally known, so the pair is unambiguous.
    #[test]
    fn a_pair_that_names_each_other_once_binds_both_ways() {
        let (switch_a, switch_b) = (Uuid::new_v4(), Uuid::new_v4());
        let (port_a, port_b) = (Uuid::new_v4(), Uuid::new_v4());

        let pairs = reciprocal_pairs(&HashMap::from([
            ((switch_a, switch_b), vec![evidenced(port_a)]),
            ((switch_b, switch_a), vec![evidenced(port_b)]),
        ]));

        assert_eq!(pairs.get(&(port_a, switch_b)), Some(&port_b));
        assert_eq!(pairs.get(&(port_b, switch_a)), Some(&port_a));
    }

    /// A LAG names the same neighbour from several ports and LLDP carries nothing saying which
    /// member faces which. Guessing would draw an arbitrary port as authoritative.
    #[test]
    fn a_lag_between_two_switches_stays_device_level() {
        let (switch_a, switch_b) = (Uuid::new_v4(), Uuid::new_v4());
        let pairs = reciprocal_pairs(&HashMap::from([
            (
                (switch_a, switch_b),
                vec![evidenced(Uuid::new_v4()), evidenced(Uuid::new_v4())],
            ),
            (
                (switch_b, switch_a),
                vec![evidenced(Uuid::new_v4()), evidenced(Uuid::new_v4())],
            ),
        ]));

        assert!(pairs.is_empty());
    }

    /// One side sees two links to the other but only one comes back. The single port is still not
    /// identified — which of the two it answers is exactly what is unknown.
    #[test]
    fn an_asymmetric_count_names_no_port_on_either_side() {
        let (switch_a, switch_b) = (Uuid::new_v4(), Uuid::new_v4());

        let pairs = reciprocal_pairs(&HashMap::from([
            (
                (switch_a, switch_b),
                vec![evidenced(Uuid::new_v4()), evidenced(Uuid::new_v4())],
            ),
            ((switch_b, switch_a), vec![evidenced(Uuid::new_v4())]),
        ]));

        assert!(pairs.is_empty());
    }

    /// The failure this column exists for: a port whose neighbour evidence has stopped arriving
    /// keeps its stored binding, and `build_neighbor_adjacency` reads that binding back as
    /// adjacency. Pairing on it would make the binding its own confirmation, so
    /// `re_examine_port_binding` would answer `Stands` for a link nothing evidences any more.
    ///
    /// The far end is deliberately still answering: in the case this was reproduced from it is the
    /// local port that went quiet, and a rule that only checked the far end would confirm it.
    #[test]
    fn a_pair_resting_on_evidence_that_stopped_arriving_does_not_pair() {
        let (switch_a, switch_b) = (Uuid::new_v4(), Uuid::new_v4());
        let (port_a, port_b) = (Uuid::new_v4(), Uuid::new_v4());

        let pairs = reciprocal_pairs(&HashMap::from([
            ((switch_a, switch_b), vec![unevidenced(port_a)]),
            ((switch_b, switch_a), vec![evidenced(port_b)]),
        ]));

        assert!(pairs.is_empty());
    }

    /// A LAG whose members have already resolved names the far end from stored bindings rather
    /// than from identifiers. Those still have to count, or the tally would see one evidenced port
    /// each way, pair them, and draw an arbitrary LAG member as authoritative — the outcome the
    /// "exactly one each way" rule exists to prevent.
    #[test]
    fn a_lag_stays_device_level_when_only_one_member_still_has_evidence() {
        let (switch_a, switch_b) = (Uuid::new_v4(), Uuid::new_v4());

        let pairs = reciprocal_pairs(&HashMap::from([
            (
                (switch_a, switch_b),
                vec![evidenced(Uuid::new_v4()), unevidenced(Uuid::new_v4())],
            ),
            (
                (switch_b, switch_a),
                vec![evidenced(Uuid::new_v4()), unevidenced(Uuid::new_v4())],
            ),
        ]));

        assert!(pairs.is_empty());
    }

    /// A neighbour that never names us back — an endpoint, a phone, a device scanned by something
    /// that does not read its LLDP — carries no reciprocal evidence at all.
    #[test]
    fn a_one_sided_adjacency_does_not_pair() {
        let (switch, endpoint) = (Uuid::new_v4(), Uuid::new_v4());

        let pairs = reciprocal_pairs(&HashMap::from([(
            (switch, endpoint),
            vec![evidenced(Uuid::new_v4())],
        )]));

        assert!(pairs.is_empty());
    }

    /// A device with one link to each of two neighbours pairs both: the rule counts ports *per
    /// neighbour pair*, not per device, or a switch with more than one uplink would never pair.
    #[test]
    fn one_port_per_neighbour_pairs_every_neighbour() {
        let (core, edge_one, edge_two) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let (core_to_one, core_to_two) = (Uuid::new_v4(), Uuid::new_v4());
        let (one_to_core, two_to_core) = (Uuid::new_v4(), Uuid::new_v4());

        let pairs = reciprocal_pairs(&HashMap::from([
            ((core, edge_one), vec![evidenced(core_to_one)]),
            ((edge_one, core), vec![evidenced(one_to_core)]),
            ((core, edge_two), vec![evidenced(core_to_two)]),
            ((edge_two, core), vec![evidenced(two_to_core)]),
        ]));

        assert_eq!(pairs.get(&(core_to_one, edge_one)), Some(&one_to_core));
        assert_eq!(pairs.get(&(core_to_two, edge_two)), Some(&two_to_core));
    }

    /// The tier runs only once the identifiers the far end advertised have failed: a port id that
    /// named a single port is authoritative and reciprocal evidence must not displace it.
    #[test]
    fn a_port_the_advertised_id_already_named_is_left_alone() {
        let named = Uuid::new_v4();
        let local = Uuid::new_v4();
        let host = Uuid::new_v4();
        let reciprocal = HashMap::from([((local, host), Uuid::new_v4())]);

        assert!(
            HostService::pair_reciprocally(
                IdentityResolution::Resolved(named),
                local,
                host,
                &reciprocal,
            )
            .is_none()
        );
    }

    /// A pairing keyed on a different remote host than the chassis ladder settled on is not
    /// evidence about this link — the composite key means the two can no longer even agree by
    /// accident on the wrong host, since a lookup against the resolved host simply misses.
    #[test]
    fn a_pairing_disagreeing_with_the_resolved_host_is_not_used() {
        let local = Uuid::new_v4();
        let reciprocal = HashMap::from([((local, Uuid::new_v4()), Uuid::new_v4())]);

        assert!(
            HostService::pair_reciprocally(
                IdentityResolution::Ambiguous,
                local,
                Uuid::new_v4(),
                &reciprocal,
            )
            .is_none()
        );
    }

    // ========================================================================
    // GH #701 characterization: shared L2 segment, 3 nodes, each naming the other two.
    // ========================================================================

    /// A router and two hosts behind a bridge, every port hearing both others — the shape from the
    /// issue. Runs `reciprocal_pairs` directly (DB-free): each device names its two neighbours on
    /// its own single uplink port (the realistic shape for a switch family that reports one
    /// chassis MAC per port, where every neighbour the reciprocal tier considers arrives on the
    /// *same* local port), so both other devices' `NamingPort`s land in the same interface's
    /// bucket — exactly the case `build_neighbor_adjacency`'s per-candidate dedup exists for.
    fn shared_segment_ports_between(
        router_port: Uuid,
        host_a_port: Uuid,
        host_b_port: Uuid,
        router: Uuid,
        host_a: Uuid,
        host_b: Uuid,
    ) -> HashMap<(Uuid, Uuid), Vec<NamingPort>> {
        HashMap::from([
            ((router, host_a), vec![evidenced(router_port)]),
            ((router, host_b), vec![evidenced(router_port)]),
            ((host_a, router), vec![evidenced(host_a_port)]),
            ((host_a, host_b), vec![evidenced(host_a_port)]),
            ((host_b, router), vec![evidenced(host_b_port)]),
            ((host_b, host_a), vec![evidenced(host_b_port)]),
        ])
    }

    /// The core characterization: running the adjacency build twice on identical input must
    /// produce identical bindings both times. Before the `(interface, remote_host)` re-keying,
    /// `reciprocal_pairs`'s old `HashMap<Uuid, (Uuid, Uuid)>` gave `router_port` one slot for two
    /// distinct neighbours — whichever pair the `HashMap` iterated last silently overwrote the
    /// other — so which of `host_a`/`host_b` `router_port` ended up paired with varied from run to
    /// run despite unchanged input. That is the exact mechanism behind "the stored neighbour flips
    /// between identical scans."
    #[test]
    fn a_three_node_shared_segment_resolves_identically_across_repeated_runs() {
        let (router, host_a, host_b) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let (router_port, host_a_port, host_b_port) =
            (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());

        let ports_between = shared_segment_ports_between(
            router_port,
            host_a_port,
            host_b_port,
            router,
            host_a,
            host_b,
        );

        let first = reciprocal_pairs(&ports_between);
        let second = reciprocal_pairs(&ports_between);

        assert_eq!(
            first, second,
            "identical input must resolve to identical bindings on every run"
        );

        // All three pairwise adjacencies must be present — the router's one port pairs with
        // *both* hosts (two entries keyed on the same interface_id, different remote hosts), not
        // just whichever one a HashMap happened to iterate last.
        assert_eq!(first.get(&(router_port, host_a)), Some(&host_a_port));
        assert_eq!(first.get(&(router_port, host_b)), Some(&host_b_port));
        assert_eq!(first.get(&(host_a_port, router)), Some(&router_port));
        assert_eq!(first.get(&(host_a_port, host_b)), Some(&host_b_port));
        assert_eq!(first.get(&(host_b_port, router)), Some(&router_port));
        assert_eq!(first.get(&(host_b_port, host_a)), Some(&host_a_port));
        assert_eq!(
            first.len(),
            6,
            "3 nodes each naming the other 2 is 6 directed pairings, all of them stable"
        );
    }
}
