//! Bespoke read/write paths for the three GH #701 tables.
//!
//! No `CrudService`/`ChildCrudService` here — both require `Entity` (`shared/services/traits.rs`),
//! which these tables deliberately don't implement (see `impl/base.rs`'s module docs). Instead:
//! `GenericChildStorage<T>` gives parent-scoped reads and a replace-all write for the disposable
//! candidates table for free, and this service hand-writes the natural-key reconciliation the two
//! resolved tables need — the same shape as `Interface::create_or_update_from_discovery`, at
//! per-adjacency-row granularity instead of per-interface.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::server::{
    interface_neighbors::r#impl::base::{
        InterfaceNeighborCandidate, InterfaceNeighborCandidateBase, InterfaceNeighborEvidence,
        InterfaceNeighborHost, InterfaceNeighborHostBase, InterfaceNeighborInterface,
        InterfaceNeighborInterfaceBase, InterfaceNeighborRow, Neighbor,
    },
    interfaces::r#impl::base::InterfaceDataComplete,
    shared::storage::{child::GenericChildStorage, filter::StorableFilter, traits::Storage},
};

pub struct InterfaceNeighborService {
    candidates: GenericChildStorage<InterfaceNeighborCandidate>,
    resolved_interfaces: GenericChildStorage<InterfaceNeighborInterface>,
    resolved_hosts: GenericChildStorage<InterfaceNeighborHost>,
}

impl InterfaceNeighborService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            candidates: GenericChildStorage::new(pool.clone()),
            resolved_interfaces: GenericChildStorage::new(pool.clone()),
            resolved_hosts: GenericChildStorage::new(pool),
        }
    }

    // ========================================================================
    // Candidates — resolution's input
    // ========================================================================

    pub async fn candidates_for_interface(
        &self,
        interface_id: &Uuid,
    ) -> Result<Vec<InterfaceNeighborCandidate>> {
        self.candidates.get_for_parent(interface_id).await
    }

    /// Every candidate in the network, in one query — for callers (topology's protocol-label
    /// derivation) that need the raw evidence rather than the resolved outcome.
    pub async fn candidates_for_network(
        &self,
        network_id: Uuid,
    ) -> Result<Vec<InterfaceNeighborCandidate>> {
        let filter =
            StorableFilter::<InterfaceNeighborCandidate>::new_from_network_ids(&[network_id]);
        self.candidates.inner().get_all(filter).await
    }

    pub async fn candidates_for_interfaces(
        &self,
        interface_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<InterfaceNeighborCandidate>>> {
        self.candidates.get_for_parents(interface_ids).await
    }

    /// Replace an interface's candidate rows with what this scan submitted.
    ///
    /// A row present in `incoming` has already passed the daemon's own per-row validation — a
    /// malformed LLDP/CDP record (missing chassis id, wrong ASN.1 type, a key no chassis column
    /// ever listed) is dropped at the source before it can become evidence at all (see
    /// `query_lldp_neighbors_for`) — so it is trusted outright here regardless of `collected`.
    /// `collected` being false for the walk as a whole no longer vetoes a row that already proved
    /// itself individually; earlier it did, which is what let one malformed row on one port veto
    /// every other interface's good row on the same device.
    ///
    /// `collected` still answers the one thing a validated row can't: what about a group this
    /// interface reported *nothing* for this scan? That silence is ambiguous — the neighbour
    /// could be gone, or the walk just never reached it — so a stored row of that shape is carried
    /// forward unchanged, `created_at` included, the same way `Interface::preserve_uncollected_data`
    /// guarded the scalar fields these candidates replaced. Scoped per interface, not per host: an
    /// interface that *did* submit fresh evidence for a group this scan has already answered the
    /// question for itself, and carrying forward stale history for it too would plant a
    /// duplicate, possibly conflicting neighbour beside the fresh one.
    pub async fn replace_candidates_from_discovery(
        &self,
        network_id: Uuid,
        interface_id: Uuid,
        incoming: Vec<InterfaceNeighborEvidence>,
        collected: InterfaceDataComplete,
    ) -> Result<()> {
        // Freshly submitted this scan: brand-new rows, `created_at` = now. `created_at` is what
        // the resolution ladder's `evidence_current` check reads (candidates carry no other
        // per-row freshness column, and are replaced wholesale rather than diffed) — so a row's
        // `created_at` must mean "last confirmed by a scan", not "last written to storage".
        let mut rows: Vec<InterfaceNeighborCandidate> = incoming
            .into_iter()
            .map(|evidence| {
                InterfaceNeighborCandidate::new(InterfaceNeighborCandidateBase::new(
                    network_id,
                    interface_id,
                    evidence,
                ))
            })
            .collect();

        // A group this scan did not finish reading, for an interface that got no fresh row of
        // that shape either: carry the *existing* rows of that shape forward unchanged,
        // `created_at` included — a walk cut short must not look like fresh evidence, or a link
        // whose neighbour walk has been failing for a month would read as just-confirmed on every
        // scan. Skipped entirely for a group this interface *did* submit fresh evidence for, so a
        // recovering port never gets its fresh neighbour joined by a stale one.
        if !collected.lldp || !collected.cdp {
            let fresh_has_lldp = rows.iter().any(|r| r.base.evidence.has_lldp_data());
            let fresh_has_cdp = rows.iter().any(|r| r.base.evidence.has_cdp_data());
            for candidate in self.candidates_for_interface(&interface_id).await? {
                let evidence = &candidate.base.evidence;
                let keep_lldp = !collected.lldp && !fresh_has_lldp && evidence.has_lldp_data();
                let keep_cdp = !collected.cdp && !fresh_has_cdp && evidence.has_cdp_data();
                if keep_lldp || keep_cdp {
                    rows.push(candidate);
                }
            }
        }

        // Replace-all by hand rather than `GenericChildStorage::save_for_parent`/`ChildStorage::
        // save_for_parent`: neither has a real implementation (the `ChildStorage` trait impl in
        // `shared/storage/child.rs` calls its own trait method — an infinite-recursion stub, not
        // dead code caught here only because nothing else calls it yet). Mirrors
        // `DependencyMemberStorage::save_for_dependency`'s delete-then-insert-in-one-transaction
        // shape instead.
        let mut tx = self.candidates.inner().begin_transaction().await?;
        let filter = StorableFilter::<InterfaceNeighborCandidate>::new_from_uuids_column(
            "interface_id",
            &[interface_id],
        );
        tx.delete_by_filter(filter).await?;
        for row in &rows {
            tx.create(row).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    // ========================================================================
    // Resolved reads — the merged read model
    // ========================================================================

    pub async fn resolved_for_interface(
        &self,
        interface_id: &Uuid,
    ) -> Result<Vec<InterfaceNeighborRow>> {
        let interfaces = self
            .resolved_interfaces
            .get_for_parent(interface_id)
            .await?;
        let hosts = self.resolved_hosts.get_for_parent(interface_id).await?;
        Ok(interfaces
            .iter()
            .map(InterfaceNeighborRow::from)
            .chain(hosts.iter().map(InterfaceNeighborRow::from))
            .collect())
    }

    pub async fn resolved_for_interfaces(
        &self,
        interface_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<InterfaceNeighborRow>>> {
        if interface_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let interfaces = self
            .resolved_interfaces
            .get_for_parents(interface_ids)
            .await?;
        let hosts = self.resolved_hosts.get_for_parents(interface_ids).await?;

        let mut result: HashMap<Uuid, Vec<InterfaceNeighborRow>> = HashMap::new();
        for (id, rows) in &interfaces {
            result
                .entry(*id)
                .or_default()
                .extend(rows.iter().map(InterfaceNeighborRow::from));
        }
        for (id, rows) in &hosts {
            result
                .entry(*id)
                .or_default()
                .extend(rows.iter().map(InterfaceNeighborRow::from));
        }
        Ok(result)
    }

    /// Every resolved adjacency in a network, pinned the same way `interfaces` is — for
    /// `TopologyContext`, which needs the whole network's merged read model rather than one
    /// interface's. `snapshot_id = None` reads live rows; `Some(id)` reads the closed copies
    /// stamped at that snapshot — the same `apply_snapshot` convention `TopologyService::
    /// get_topology_data` already uses for every other entity type.
    pub async fn resolved_for_network(
        &self,
        network_id: Uuid,
        snapshot_id: Option<Uuid>,
    ) -> Result<Vec<InterfaceNeighborRow>> {
        let base_interfaces_filter =
            StorableFilter::<InterfaceNeighborInterface>::new_from_network_ids(&[network_id]);
        let base_hosts_filter =
            StorableFilter::<InterfaceNeighborHost>::new_from_network_ids(&[network_id]);
        let (interfaces_filter, hosts_filter) = match snapshot_id {
            None => (base_interfaces_filter.live(), base_hosts_filter.live()),
            Some(id) => (
                base_interfaces_filter.snapshot_id(&id),
                base_hosts_filter.snapshot_id(&id),
            ),
        };

        let interfaces = self
            .resolved_interfaces
            .inner()
            .get_all(interfaces_filter)
            .await?;
        let hosts = self.resolved_hosts.inner().get_all(hosts_filter).await?;

        Ok(interfaces
            .iter()
            .map(InterfaceNeighborRow::from)
            .chain(hosts.iter().map(InterfaceNeighborRow::from))
            .collect())
    }

    // ========================================================================
    // Resolution write path — natural-key reconciliation
    // ========================================================================

    /// Reconcile one local interface's resolved adjacency set against what one resolution pass
    /// concluded.
    ///
    /// `desired` is every neighbour this pass resolved for `interface_id` — the reciprocal tier
    /// and the chassis/CDP ladder have already run and already deduped by remote host, so this is
    /// purely a diff-and-write against storage. A row not named by `desired` is deleted outright:
    /// ordinary discovery writes retire what stopped being true the same way `create.rs`'s
    /// end-of-scan interface prune does (a hard `delete`, not a soft SCD2 close) — closed
    /// historical copies exist only where a taken `Snapshot`'s close-and-clone produced them
    /// (`SnapshotService::run_close_and_clone`), which is a different, coarser-grained mechanism
    /// than a per-scan reconcile. A row whose `neighbor_seen_at` is unchanged is left untouched
    /// entirely — the per-candidate analogue of `persist_neighbor`'s write-only-on-change guard,
    /// which is what keeps a stable network from paying an SCD2-adjacent write per link per scan.
    pub async fn reconcile_interface_neighbors(
        &self,
        network_id: Uuid,
        interface_id: Uuid,
        desired: &[(Neighbor, Option<DateTime<Utc>>)],
        scan_time: DateTime<Utc>,
        discovery_id: Option<Uuid>,
    ) -> Result<()> {
        let existing_interfaces = self
            .resolved_interfaces
            .get_for_parent(&interface_id)
            .await?;
        let existing_hosts = self.resolved_hosts.get_for_parent(&interface_id).await?;

        let desired_interface_ids: HashSet<Uuid> = desired
            .iter()
            .filter_map(|(n, _)| n.interface_id())
            .collect();
        let desired_host_ids: HashSet<Uuid> = desired
            .iter()
            .filter_map(|(n, _)| match n {
                Neighbor::Host(id) => Some(*id),
                Neighbor::Interface(_) => None,
            })
            .collect();

        for row in &existing_interfaces {
            if !desired_interface_ids.contains(&row.base.neighbor_interface_id) {
                self.resolved_interfaces.inner().delete(&row.id).await?;
            }
        }
        for row in &existing_hosts {
            if !desired_host_ids.contains(&row.base.neighbor_host_id) {
                self.resolved_hosts.inner().delete(&row.id).await?;
            }
        }

        for (neighbor, neighbor_seen_at) in desired {
            match neighbor {
                Neighbor::Interface(neighbor_interface_id) => {
                    let existing = existing_interfaces
                        .iter()
                        .find(|r| r.base.neighbor_interface_id == *neighbor_interface_id);
                    match existing {
                        Some(existing) if existing.base.neighbor_seen_at == *neighbor_seen_at => {
                            // Unchanged: no write, matching `persist_neighbor`'s no-op branch.
                        }
                        Some(existing) => {
                            let mut updated = existing.clone();
                            updated.base.neighbor_seen_at = *neighbor_seen_at;
                            updated.refresh_scan_timestamps(scan_time);
                            if let Some(id) = discovery_id {
                                updated.last_discovery_id = Some(id);
                            }
                            self.resolved_interfaces
                                .inner()
                                .update(&mut updated)
                                .await?;
                        }
                        None => {
                            let mut created =
                                InterfaceNeighborInterface::new(InterfaceNeighborInterfaceBase {
                                    network_id,
                                    interface_id,
                                    neighbor_interface_id: *neighbor_interface_id,
                                    neighbor_seen_at: *neighbor_seen_at,
                                });
                            created.originate_scan_timestamps(scan_time);
                            created.last_seen_at = scan_time;
                            created.first_discovery_id = discovery_id;
                            created.last_discovery_id = discovery_id;
                            self.resolved_interfaces.inner().create(&created).await?;
                        }
                    }
                }
                Neighbor::Host(neighbor_host_id) => {
                    let existing = existing_hosts
                        .iter()
                        .find(|r| r.base.neighbor_host_id == *neighbor_host_id);
                    match existing {
                        Some(existing) if existing.base.neighbor_seen_at == *neighbor_seen_at => {}
                        Some(existing) => {
                            let mut updated = existing.clone();
                            updated.base.neighbor_seen_at = *neighbor_seen_at;
                            updated.refresh_scan_timestamps(scan_time);
                            if let Some(id) = discovery_id {
                                updated.last_discovery_id = Some(id);
                            }
                            self.resolved_hosts.inner().update(&mut updated).await?;
                        }
                        None => {
                            let mut created =
                                InterfaceNeighborHost::new(InterfaceNeighborHostBase {
                                    network_id,
                                    interface_id,
                                    neighbor_host_id: *neighbor_host_id,
                                    neighbor_seen_at: *neighbor_seen_at,
                                });
                            created.originate_scan_timestamps(scan_time);
                            created.last_seen_at = scan_time;
                            created.first_discovery_id = discovery_id;
                            created.last_discovery_id = discovery_id;
                            self.resolved_hosts.inner().create(&created).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Every candidate row with no matching row in either resolved table — the multi-row
    /// replacement for the old `unresolved_lldp_port_in_network` filter, which compared a single
    /// scalar `neighbor_interface_id`/`neighbor_host_id` against `NULL`. Used by the admin/debug
    /// unresolved-evidence view.
    pub async fn unresolved_candidates_for_network(
        &self,
        network_id: Uuid,
    ) -> Result<Vec<InterfaceNeighborCandidate>> {
        let filter =
            StorableFilter::<InterfaceNeighborCandidate>::new_from_network_ids(&[network_id]);
        let candidates = self.candidates.inner().get_all(filter).await?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let interface_ids: Vec<Uuid> = candidates
            .iter()
            .map(|c| c.base.interface_id)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        let resolved = self.resolved_for_interfaces(&interface_ids).await?;

        Ok(candidates
            .into_iter()
            .filter(|c| {
                resolved
                    .get(&c.base.interface_id)
                    .is_none_or(|rows| rows.is_empty())
            })
            .collect())
    }
}
