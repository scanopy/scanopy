use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

use crate::server::networks::r#impl::Network;
use crate::server::networks::service::NetworkService;
use crate::server::organizations::service::OrganizationService;
use crate::server::shared::events::bus::EventBus;
use crate::server::shared::services::traits::{CrudService, EventBusService};
use crate::server::shared::storage::{
    filter::StorableFilter,
    generic::GenericPostgresStorage,
    snapshot::{FkMaps, Snapshotable},
    traits::{Storable, Storage},
};
use crate::server::snapshots::types::base::Snapshot;
use crate::server::tags::entity_tags::EntityTagService;

use crate::server::hosts::r#impl::base::Host;
use crate::server::services::r#impl::base::Service;
use crate::server::subnets::r#impl::base::Subnet;

/// Entities whose `virtualization` carries a `service_id` pointing at another
/// tracked `Service`: `Host` → hypervisor service, `Service` → container-runtime
/// service, `Subnet` → the docker daemon that owns the bridge.
///
/// These references can't be remapped in the per-row `remap_fks_for_clone` pass:
/// subnets and hosts clone before services, and services self-reference, so
/// `FkMaps::services` isn't populated yet. They're patched in a dedicated
/// post-clone pass once the full service map exists (see
/// `remap_virtualization_service_ids`).
trait VirtualizationServiceRef {
    fn virtualization_service_id(&self) -> Option<Uuid>;
    fn set_virtualization_service_id(&mut self, id: Uuid);
}

// Each of these used to reach into a JSONB payload and could only set an id when a payload was
// already there. The column is set independently of any metadata, so a clone that carries a
// reference but no metadata is now remappable too.
impl VirtualizationServiceRef for Host {
    fn virtualization_service_id(&self) -> Option<Uuid> {
        self.base.virtualization_service_id
    }
    fn set_virtualization_service_id(&mut self, id: Uuid) {
        self.base.virtualization_service_id = Some(id);
    }
}

impl VirtualizationServiceRef for Service {
    fn virtualization_service_id(&self) -> Option<Uuid> {
        self.base.virtualization_service_id
    }
    fn set_virtualization_service_id(&mut self, id: Uuid) {
        self.base.virtualization_service_id = Some(id);
    }
}

impl VirtualizationServiceRef for Subnet {
    fn virtualization_service_id(&self) -> Option<Uuid> {
        self.base.virtualization_service_id
    }
    fn set_virtualization_service_id(&mut self, id: Uuid) {
        self.base.virtualization_service_id = Some(id);
    }
}

/// Network snapshots: close-and-clone the live row set for a network at a
/// single timestamp inside one transaction. Also acts as the `CrudService`
/// for the `Snapshot` entity so the standard handlers can read/list/delete
/// snapshot rows.
pub struct SnapshotService {
    pool: Arc<PgPool>,
    storage: Arc<GenericPostgresStorage<Snapshot>>,
    event_bus: Arc<EventBus>,
    network_service: Arc<NetworkService>,
    organization_service: Arc<OrganizationService>,
}

impl SnapshotService {
    pub fn new(
        pool: Arc<PgPool>,
        storage: Arc<GenericPostgresStorage<Snapshot>>,
        event_bus: Arc<EventBus>,
        network_service: Arc<NetworkService>,
        organization_service: Arc<OrganizationService>,
    ) -> Arc<Self> {
        Arc::new(Self {
            pool,
            storage,
            event_bus,
            network_service,
            organization_service,
        })
    }
}

impl EventBusService<Snapshot> for SnapshotService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Snapshot) -> Option<Uuid> {
        Some(entity.base.network_id)
    }

    fn get_organization_id(&self, _entity: &Snapshot) -> Option<Uuid> {
        None
    }
}

#[async_trait]
impl CrudService<Snapshot> for SnapshotService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Snapshot>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl SnapshotService {
    /// Synchronous orchestration of one network snapshot at `taken_at`.
    ///
    /// Caller (the manual-snapshot API handler or future scheduled-snapshot
    /// machinery) is responsible for `DiscoveryService::try_acquire_network_for_snapshot`
    /// before calling, and `release_network_for_snapshot` after — regardless
    /// of result.
    ///
    /// All twelve network-scoped Snapshotable entity types are processed
    /// parents-first so child rows can remap their FK columns to the closed
    /// parent ids via [`FkMaps`]. The whole sequence runs in a single
    /// `sqlx::Transaction`; if any step fails, nothing is committed.
    pub async fn run_close_and_clone(
        &self,
        network_id: Uuid,
        taken_at: DateTime<Utc>,
        snapshot_id: Uuid,
    ) -> Result<()> {
        use crate::server::bindings::r#impl::base::Binding;
        use crate::server::dependencies::dependency_members::DependencyMemberRecord;
        use crate::server::dependencies::r#impl::base::Dependency;
        use crate::server::interface_neighbors::r#impl::base::{
            InterfaceNeighborHost, InterfaceNeighborInterface,
        };
        use crate::server::interfaces::r#impl::base::Interface;
        use crate::server::ip_addresses::r#impl::base::IPAddress;
        use crate::server::ports::r#impl::base::Port;
        use crate::server::tags::entity_tags::EntityTag;
        use crate::server::vlans::r#impl::base::Vlan;
        use crate::server::vlans::r#impl::subnet_vlans::SubnetVlanRecord;

        let mut tx = self.pool.begin().await?;
        let mut maps = FkMaps::default();

        // Top-level network-scoped entities (no within-tracked FKs to remap).
        let subnet_map = close_and_clone_for::<Subnet>(
            &mut tx,
            network_filter::<Subnet>(network_id),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;
        maps.subnets = subnet_map;

        let vlan_map = close_and_clone_for::<Vlan>(
            &mut tx,
            network_filter::<Vlan>(network_id),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;
        maps.vlans = vlan_map;

        let host_map = close_and_clone_for::<Host>(
            &mut tx,
            network_filter::<Host>(network_id),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;
        maps.hosts = host_map;

        // Children of hosts/subnets/vlans. Remap their FK columns using the
        // parent maps populated above.
        let ip_map = close_and_clone_for::<IPAddress>(
            &mut tx,
            network_filter::<IPAddress>(network_id),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;
        maps.ip_addresses = ip_map;

        // Ports filter through host_id (Port has no network_id column).
        let host_ids: Vec<Uuid> = maps.hosts.keys().copied().collect();
        let port_map = close_and_clone_for::<Port>(
            &mut tx,
            StorableFilter::<Port>::new_from_uuids_column("host_id", &host_ids).live(),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;
        maps.ports = port_map;

        let service_map = close_and_clone_for::<Service>(
            &mut tx,
            network_filter::<Service>(network_id),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;
        maps.services = service_map;

        // Remap the `virtualization.service_id` references that point at other
        // services (Host→hypervisor, Service→container-runtime,
        // Subnet→docker-bridge owner). These couldn't be remapped in the
        // per-row clone pass because the full service map only exists now.
        // Without this, hypervisor / container-runtime groupings and
        // docker-bridge subnet↔host connections are absent from snapshots.
        remap_virtualization_service_ids::<Host>(&mut tx, snapshot_id, &maps.services).await?;
        remap_virtualization_service_ids::<Service>(&mut tx, snapshot_id, &maps.services).await?;
        remap_virtualization_service_ids::<Subnet>(&mut tx, snapshot_id, &maps.services).await?;

        // Interfaces filter through host_id (Interface has no network_id).
        let interface_map = close_and_clone_for::<Interface>(
            &mut tx,
            StorableFilter::<Interface>::new_from_uuids_column("host_id", &host_ids).live(),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;
        maps.interfaces = interface_map;

        // GH #701: resolved neighbour adjacencies, filtered through interface_id (neither table
        // has a network_id-free shortcut — both are children of `interfaces`, same as ports).
        // Both FKs (`interface_id`, `neighbor_interface_id`/`neighbor_host_id`) remap in the
        // per-row `remap_fks_for_clone` pass against `maps.interfaces`/`maps.hosts`, which are
        // both already populated by this point — unlike the old `Interface.neighbor` self-
        // reference this replaced, neither resolved table self-references, so there is no
        // `own_clone_ref` two-phase dance to run here.
        let interface_ids: Vec<Uuid> = maps.interfaces.keys().copied().collect();
        let _ = close_and_clone_for::<InterfaceNeighborInterface>(
            &mut tx,
            StorableFilter::<InterfaceNeighborInterface>::new_from_uuids_column(
                "interface_id",
                &interface_ids,
            )
            .live(),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;
        let _ = close_and_clone_for::<InterfaceNeighborHost>(
            &mut tx,
            StorableFilter::<InterfaceNeighborHost>::new_from_uuids_column(
                "interface_id",
                &interface_ids,
            )
            .live(),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;

        // Bindings filter through service_id (Binding has no network_id).
        // BINDINGS must come before DEPENDENCY_MEMBERS so dep_member's
        // optional binding_id can be remapped.
        let service_ids: Vec<Uuid> = maps.services.keys().copied().collect();
        let binding_map = close_and_clone_for::<Binding>(
            &mut tx,
            StorableFilter::<Binding>::new_from_uuids_column("service_id", &service_ids).live(),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;
        maps.bindings = binding_map;

        // SubnetVlan junction filters through subnet_id.
        let subnet_ids: Vec<Uuid> = maps.subnets.keys().copied().collect();
        let _ = close_and_clone_for::<SubnetVlanRecord>(
            &mut tx,
            StorableFilter::<SubnetVlanRecord>::new_from_uuids_column("subnet_id", &subnet_ids)
                .live(),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;

        let dependency_map = close_and_clone_for::<Dependency>(
            &mut tx,
            network_filter::<Dependency>(network_id),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;
        maps.dependencies = dependency_map;

        // DependencyMembers filter through dependency_id.
        let dependency_ids: Vec<Uuid> = maps.dependencies.keys().copied().collect();
        let _ = close_and_clone_for::<DependencyMemberRecord>(
            &mut tx,
            StorableFilter::<DependencyMemberRecord>::new_from_uuids_column(
                "dependency_id",
                &dependency_ids,
            )
            .live(),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;

        // EntityTags: filter to network-scoped entity types only. Org-scoped
        // variants (Daemon, User, DaemonApiKey, UserApiKey, etc.) are not
        // cloned at network snapshot. The set of network-scoped entity ids
        // is the union of host/service/subnet/dependency live ids that
        // were just cloned (via maps).
        let entity_ids: Vec<Uuid> = maps
            .hosts
            .keys()
            .chain(maps.services.keys())
            .chain(maps.subnets.keys())
            .chain(maps.dependencies.keys())
            .copied()
            .collect();
        let _ = close_and_clone_for::<EntityTag>(
            &mut tx,
            StorableFilter::<EntityTag>::new_from_uuids_column("entity_id", &entity_ids).live(),
            taken_at,
            snapshot_id,
            &maps,
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Daily entry point. Iterates all orgs, resolves retention via
    /// `BillingPlan::snapshot_retention_days(env_override)`, delegates per-org.
    /// Idempotent.
    pub async fn run_retention(&self, env_override: Option<u32>) -> Result<()> {
        let orgs = self
            .organization_service
            .get_all(StorableFilter::new_for_retention_sweep())
            .await?;

        for org in orgs {
            let plan = org
                .base
                .plan
                .unwrap_or_else(crate::server::billing::plans::get_free_plan);
            let days = plan.snapshot_retention_days(env_override);
            if let Err(e) = self.trim_org(org.id, days).await {
                tracing::error!(
                    organization_id = %org.id,
                    error = ?e,
                    "snapshot retention trim_org failed",
                );
            }
        }

        Ok(())
    }

    /// Per-org retention trim. Deletes snapshots older than the cutoff. The
    /// cascade FK on closed entity rows + topology rows reaps everything tied
    /// to the deleted snapshots automatically. Live rows (snapshot_id IS NULL)
    /// are untouched.
    pub async fn trim_org(&self, org_id: Uuid, retention_days: u32) -> Result<()> {
        let networks = self
            .network_service
            .get_all(StorableFilter::<Network>::new_from_org_id(&org_id))
            .await?;
        let network_ids: Vec<Uuid> = networks.into_iter().map(|n| n.id).collect();

        if network_ids.is_empty() {
            return Ok(());
        }

        // `retention_days == 0` means the plan doesn't grant snapshot retention at
        // all — every existing snapshot is older than the cutoff and should be
        // deleted. (TakeSnapshotFeature treats 0 the same way: snapshots aren't
        // a feature on this plan, new ones are blocked, and the sweep is what
        // clears any inherited from a higher tier.)
        let cutoff = Utc::now() - Duration::days(retention_days as i64);
        let filter =
            StorableFilter::<Snapshot>::new_from_network_ids(&network_ids).taken_at_lt(cutoff);
        self.storage.delete_by_filter(filter).await?;

        Ok(())
    }
}

/// Standard "live rows on this network" filter. Used by the entity types
/// that carry a `network_id` column directly (subnets, vlans, hosts,
/// ip_addresses, services, dependencies). Junction tables that lack
/// `network_id` are filtered through their parents above.
fn network_filter<T: Storable>(network_id: Uuid) -> StorableFilter<T> {
    StorableFilter::<T>::new_from_network_ids(&[network_id]).live()
}

/// Generic close-and-clone for one Snapshotable entity type within a shared
/// transaction. Returns the per-type live-id → closed-id map for downstream
/// children to consult via `FkMaps`.
/// Post-clone fixup: rewrite `virtualization.service_id` on this snapshot's
/// just-cloned closed rows of type `T` to point at closed-copy services.
///
/// Runs after the full `FkMaps::services` exists. Re-reads the closed rows by
/// `snapshot_id` (the same filter the read path uses), remaps any reference
/// present in `services`, and writes back only the changed rows. The
/// raw-stamped `snapshot_id` column isn't part of `to_params`, so
/// `update_many_in_tx` leaves it (and the SCD2 columns) intact.
async fn remap_virtualization_service_ids<T>(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    snapshot_id: Uuid,
    services: &HashMap<Uuid, Uuid>,
) -> Result<()>
where
    T: Snapshotable + VirtualizationServiceRef + std::fmt::Display,
{
    let rows = GenericPostgresStorage::<T>::get_all_in_tx(
        StorableFilter::<T>::new_from_snapshot_id(&snapshot_id),
        tx,
    )
    .await?;

    let mut changed: Vec<T> = Vec::new();
    for mut row in rows {
        if remap_virtualization_service_id(&mut row, services) {
            changed.push(row);
        }
    }

    if !changed.is_empty() {
        GenericPostgresStorage::<T>::update_many_in_tx(&changed, tx).await?;
    }

    Ok(())
}

/// Rewrite a single row's `virtualization.service_id` from its live id to the
/// closed-copy id via `services` (live → closed). Returns whether the row
/// changed: `false` when it has no virtualization ref, or the ref isn't a
/// tracked/cloned service (left as-is). Pure so it's unit-testable without a DB.
fn remap_virtualization_service_id<T: VirtualizationServiceRef>(
    row: &mut T,
    services: &HashMap<Uuid, Uuid>,
) -> bool {
    if let Some(live_service_id) = row.virtualization_service_id()
        && let Some(closed_service_id) = services.get(&live_service_id)
    {
        row.set_virtualization_service_id(*closed_service_id);
        return true;
    }
    false
}

/// How many unpreserved references [`report_unpreserved_self_refs`] names before
/// eliding the rest. Same cap the LLDP resolver's `report_unresolved` uses: a
/// list that simply stops reads as though that was all of them.
const MAX_LISTED_UNPRESERVED: usize = 10;

/// Rewrite the closed rows' self-references from live ids to their closed
/// copies, using `mapping` (live → closed for this type).
///
/// Returns the rows that changed — the only ones worth writing back — and the
/// `(closed row id, live reference)` pairs whose target has no closed copy in
/// this snapshot, so the caller can report them. Pure, so the pass structure is
/// testable without a database.
fn remap_self_refs<T: Snapshotable>(
    closed: &mut [T],
    mapping: &HashMap<Uuid, Uuid>,
) -> (Vec<T>, Vec<(Uuid, Uuid)>) {
    let mut remapped = Vec::new();
    let mut unpreserved = Vec::new();

    for row in closed.iter_mut() {
        let Some(live_ref) = row.own_clone_ref() else {
            continue;
        };
        match mapping.get(&live_ref) {
            Some(closed_ref) => {
                row.set_own_clone_ref(*closed_ref);
                remapped.push(row.clone());
            }
            // No closed copy in this snapshot — a neighbour on another network,
            // say. The live id stays, which is the only value that satisfies the
            // FK, but the snapshot's L2 read cannot resolve it and drops the
            // edge. Reported rather than left to be inferred from a link that
            // silently isn't drawn.
            None => unpreserved.push((row.id_value(), live_ref)),
        }
    }

    (remapped, unpreserved)
}

/// Name the self-references this snapshot could not preserve.
fn report_unpreserved_self_refs<T: Snapshotable>(snapshot_id: Uuid, unpreserved: &[(Uuid, Uuid)]) {
    if unpreserved.is_empty() {
        return;
    }

    let listed: Vec<String> = unpreserved
        .iter()
        .take(MAX_LISTED_UNPRESERVED)
        .map(|(row_id, live_ref)| format!("{row_id}→{live_ref}"))
        .collect();
    let elided = unpreserved.len().saturating_sub(listed.len());

    tracing::warn!(
        snapshot_id = %snapshot_id,
        entity = T::table_name(),
        count = unpreserved.len(),
        elided,
        references = %listed.join(", "),
        "Snapshot could not preserve every self-reference: the target has no closed copy in this \
         snapshot, so the captured row keeps a live id the snapshot read path cannot resolve. For \
         interfaces this is an L2 link that will be missing from the snapshot's topology.",
    );
}

/// Close-and-clone every live row matching `filter`: insert a closed copy of
/// each, stamp it with `snapshot_id`, rewrite the copies' self-references to
/// point at each other, and advance the live rows' `valid_from`.
///
/// Returns the live-id → closed-id map so downstream child types can remap their
/// own FK columns through [`FkMaps`].
async fn close_and_clone_for<T>(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    filter: StorableFilter<T>,
    taken_at: DateTime<Utc>,
    snapshot_id: Uuid,
    parent_maps: &FkMaps,
) -> Result<HashMap<Uuid, Uuid>>
where
    T: Snapshotable + std::fmt::Display,
{
    let live = GenericPostgresStorage::<T>::get_all_in_tx(filter, tx).await?;

    if live.is_empty() {
        return Ok(HashMap::new());
    }

    let mut closed: Vec<T> = Vec::with_capacity(live.len());
    let mut closed_ids: Vec<Uuid> = Vec::with_capacity(live.len());
    let mut mapping: HashMap<Uuid, Uuid> = HashMap::with_capacity(live.len());

    for original in &live {
        let mut copy = original.make_closed_copy(taken_at);
        let new_id = Uuid::new_v4();
        copy.set_id_value(new_id);
        copy.remap_fks_for_clone(parent_maps);
        mapping.insert(original.id_value(), new_id);
        closed_ids.push(new_id);
        closed.push(copy);
    }

    // Inserted carrying their *original* self-references, which point at live
    // rows that already exist. `create_many_in_tx` splits the batch into one
    // statement per `MAX_BIND_PARAMS / cols_per_row` rows and the FK is
    // `NOT DEFERRABLE`, so a closed id written here dangles whenever its target
    // lands in a later chunk (GH #687). They are rewritten below instead.
    GenericPostgresStorage::<T>::create_many_in_tx(&closed, tx).await?;

    // Stamp snapshot_id on the just-inserted closed rows. Single parameterized
    // UPDATE per entity type — extending each Snapshotable struct + Storable
    // impl with a snapshot_id field would be 13 files of churn for a column
    // with no other read/write path. Confined to this transaction.
    let stamp_sql = format!(
        "UPDATE {} SET snapshot_id = $1 WHERE id = ANY($2)",
        T::table_name()
    );
    sqlx::query(&stamp_sql)
        .bind(snapshot_id)
        .bind(&closed_ids)
        .execute(&mut **tx)
        .await?;

    // Every closed row of this type is now in the table, so a reference to any
    // of them satisfies the FK regardless of which insert chunk it landed in.
    // `snapshot_id` isn't in `to_params`, so `update_many_in_tx` leaves the
    // stamp above and the SCD2 columns intact.
    let (remapped, unpreserved) = remap_self_refs(&mut closed, &mapping);
    GenericPostgresStorage::<T>::update_many_in_tx(&remapped, tx).await?;
    report_unpreserved_self_refs::<T>(snapshot_id, &unpreserved);

    let advanced_live: Vec<T> = live
        .into_iter()
        .map(|mut e| {
            e.set_valid_from(taken_at);
            e
        })
        .collect();
    GenericPostgresStorage::<T>::update_many_in_tx(&advanced_live, tx).await?;

    Ok(mapping)
}

#[cfg(test)]
mod virtualization_remap_tests {
    use super::*;
    use crate::server::hosts::r#impl::base::HostBase;
    use crate::server::services::r#impl::base::ServiceBase;
    use crate::server::subnets::r#impl::base::SubnetBase;

    fn host_with(service_id: Option<Uuid>) -> Host {
        Host::new(HostBase {
            virtualization_service_id: service_id,
            ..Default::default()
        })
    }

    fn service_with(service_id: Option<Uuid>) -> Service {
        <Service as Storable>::new(ServiceBase {
            virtualization_service_id: service_id,
            ..Default::default()
        })
    }

    fn subnet_with(service_id: Option<Uuid>) -> Subnet {
        <Subnet as Storable>::new(SubnetBase {
            virtualization_service_id: service_id,
            ..Default::default()
        })
    }

    #[test]
    fn remaps_live_service_ref_to_closed_for_all_three_entities() {
        let live = Uuid::new_v4();
        let closed = Uuid::new_v4();
        let services = HashMap::from([(live, closed)]);

        let mut host = host_with(Some(live));
        assert!(remap_virtualization_service_id(&mut host, &services));
        assert_eq!(host.virtualization_service_id(), Some(closed));

        let mut service = service_with(Some(live));
        assert!(remap_virtualization_service_id(&mut service, &services));
        assert_eq!(service.virtualization_service_id(), Some(closed));

        let mut subnet = subnet_with(Some(live));
        assert!(remap_virtualization_service_id(&mut subnet, &services));
        assert_eq!(subnet.virtualization_service_id(), Some(closed));
    }

    #[test]
    fn leaves_untracked_ref_unchanged() {
        // Service ref not present in the map (e.g. not cloned) is left as-is.
        let services: HashMap<Uuid, Uuid> = HashMap::new();
        let untracked = Uuid::new_v4();

        let mut host = host_with(Some(untracked));
        assert!(!remap_virtualization_service_id(&mut host, &services));
        assert_eq!(host.virtualization_service_id(), Some(untracked));
    }

    #[test]
    fn leaves_entity_without_virtualization_unchanged() {
        let services = HashMap::from([(Uuid::new_v4(), Uuid::new_v4())]);

        let mut host = host_with(None);
        assert!(!remap_virtualization_service_id(&mut host, &services));
        assert_eq!(host.virtualization_service_id(), None);
    }
}

// `self_ref_remap_tests` (a `remap_self_refs`/`own_clone_ref` suite fixtured on `Interface`) was
// removed here: GH #701 moved `Interface.neighbor` — the only self-reference any type in this
// codebase ever had — into `interface_neighbor_interfaces`, an FK to a *different* table
// (`interfaces`), not a self-reference. No type overrides `Snapshotable::own_clone_ref` any more,
// so `remap_self_refs`/`report_unpreserved_self_refs` below are exercised by nothing today. Left
// in place (rather than deleted) for the next entity that genuinely self-references; write its
// tests against that real type when it exists, the same way this suite was written against
// `Interface`, the one real case, rather than a synthetic stand-in.
