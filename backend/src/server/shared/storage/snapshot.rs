//! Snapshotable / DiscoveryTracked traits + FkMaps.
//!
//! `Snapshotable` is the SCD2 (slowly-changing-dimension type 2) substrate:
//! every implementing row has `valid_from` / `valid_to` / `lineage_id`.
//! Live rows have `valid_to IS NULL` and `lineage_id IS NULL`. Closed
//! historical copies have a synthetic id, `lineage_id` pointing at the
//! original live row's id, and a `valid_to` timestamp.
//!
//! `DiscoveryTracked` extends `Snapshotable` with audit columns populated by
//! daemon discovery: `last_seen_at` (refreshed on every successful natural-key
//! match) plus FK columns to `discoveries(id)` for the discovery that first
//! saw the entity and the discovery that last touched it.
//!
//! `FkMaps` carries the per-entity-type live-id → closed-id mapping populated
//! parents-first during snapshot close-and-clone. Children read from it in
//! `Snapshotable::remap_fks_for_clone` to rewrite their within-tracked FK
//! columns to point at closed counterparts (rather than at live rows whose
//! data has since moved on).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::storage::traits::Storable;
use crate::server::shared::types::entities::EntityFreshness;

/// SCD2 row lifecycle accessors. Shared by entities (Host, Service, …) and
/// junction tables (subnet_vlans, dependency_members, entity_tags) that
/// participate in site snapshots or per-action close-and-clone.
pub trait Snapshotable: Storable {
    fn id_value(&self) -> Uuid;
    fn set_id_value(&mut self, id: Uuid);

    fn valid_from(&self) -> DateTime<Utc>;
    fn valid_to(&self) -> Option<DateTime<Utc>>;
    fn lineage_id(&self) -> Option<Uuid>;

    fn set_valid_from(&mut self, t: DateTime<Utc>);
    fn set_valid_to(&mut self, t: Option<DateTime<Utc>>);
    fn set_lineage_id(&mut self, id: Option<Uuid>);

    /// Build a closed historical copy of this row at `close_at`. The caller
    /// assigns a new id to the returned copy via `set_id_value` before INSERT.
    /// Used by both snapshot-driven cloning (SnapshotService) and per-action
    /// close-and-clone (TagService::update via SnapshotMutator blanket impl).
    fn make_closed_copy(&self, close_at: DateTime<Utc>) -> Self {
        let mut closed = self.clone();
        closed.set_lineage_id(Some(self.id_value()));
        closed.set_valid_to(Some(close_at));
        // valid_from preserved (the closed row covers [valid_from, close_at]).
        closed
    }

    /// Per-entity FK remapping during snapshot-driven cloning. Each impl
    /// rewrites any of its FK columns that point at other tracked entities
    /// using the supplied id maps (live_id → closed_id, populated parents-
    /// first by the SnapshotService orchestrator).
    ///
    /// Default: no FKs to remap. Used by hosts, subnets, vlans (top-level
    /// entities with no within-tracked FKs).
    fn remap_fks_for_clone(&mut self, _maps: &FkMaps) {}

    /// This row's *self*-reference — an FK column pointing at another row of the
    /// **same** type — as it stands, or `None` for types that have none.
    ///
    /// A self-reference cannot be rewritten in the per-row `remap_fks_for_clone`
    /// pass (the target may not be cloned yet), and it cannot be rewritten
    /// before the INSERT either: `create_many_in_tx` splits a batch into one
    /// statement per `MAX_BIND_PARAMS / cols_per_row` rows, and the FK is
    /// `NOT DEFERRABLE`, so it is checked at the end of *each* statement. A
    /// closed id written before the insert therefore dangles whenever its target
    /// lands in a later chunk (GH #687). The clone is inserted carrying its
    /// original live reference — a row that already exists, so the INSERT is
    /// valid however it chunks — and `close_and_clone_for` rewrites it once
    /// every closed row of this type is in the table.
    ///
    /// Ordering the batch instead is not an option: reciprocal LLDP pairs are
    /// cycles (A→B, B→A), so no topological order exists.
    ///
    /// Default: no self-references. Overridden by `Interface` (LLDP/CDP
    /// `neighbor` pointing at another interface).
    fn own_clone_ref(&self) -> Option<Uuid> {
        None
    }

    /// Point this row's self-reference at `id`. Only called for types that
    /// return `Some` from [`Snapshotable::own_clone_ref`].
    fn set_own_clone_ref(&mut self, _id: Uuid) {}
}

/// Accessors for the discovery-driven audit columns.
///
/// `last_seen_at` advances on every successful natural-key match by daemon
/// discovery. `last_discovery_id` and `first_discovery_id` are populated
/// post-terminal by per-entity-service subscribers on the in-memory
/// `EntityOperation::Created` event published for the historical Discovery
/// row, whose scope carries the full `Entity::Discovery(...)` struct
/// including `run_type::Historical { results: { scanned, .. } }`.
///
/// The `Entity` supertrait bound exists so the default `stamp_for_scan`
/// can call `set_created_at` / `set_updated_at`. Every type that's
/// `DiscoveryTracked` in this codebase is also a user-visible `Entity`
/// (Host, IPAddress, Port, Service, Interface, Binding, Subnet, Vlan).
/// Junction tables (SubnetVlan, EntityTag, DependencyMember) are
/// `Snapshotable` but never `DiscoveryTracked`.
pub trait DiscoveryTracked: Snapshotable + crate::server::shared::storage::traits::Entity {
    fn last_seen_at(&self) -> DateTime<Utc>;
    fn last_discovery_id(&self) -> Option<Uuid>;
    fn first_discovery_id(&self) -> Option<Uuid>;

    fn set_last_seen_at(&mut self, t: DateTime<Utc>);
    fn set_last_discovery_id(&mut self, id: Option<Uuid>);
    fn set_first_discovery_id(&mut self, id: Option<Uuid>);

    /// Refresh-style timestamps that advance on **every** observation of
    /// this entity (whether new insert or upsert). Sets `last_seen_at` and
    /// `updated_at` to `scan_time`. Always safe to call; the upsert logic
    /// reads `last_seen_at` from the incoming entity to refresh the
    /// existing live row's freshness signal.
    fn refresh_scan_timestamps(&mut self, scan_time: DateTime<Utc>) {
        self.set_last_seen_at(scan_time);
        self.set_updated_at(scan_time);
    }

    /// Origin-style timestamps that anchor a row's lifecycle when it is
    /// first inserted. Sets `created_at` and `valid_from` to `scan_time`.
    ///
    /// Conceptually one-shot: a row's "when it became live" only happens
    /// once. The discovery handler calls this on incoming entities even
    /// though only the new-insert branch should observe it — the upsert
    /// branch explicitly preserves the existing row's `created_at` /
    /// `valid_from` (it never copies these from the incoming entity), so
    /// stamping here is harmless in the matched case and correct in the
    /// new-insert case. Future changes to the upsert logic must continue
    /// to preserve these on the existing row; the method-level split here
    /// is the documentation that flags this.
    fn originate_scan_timestamps(&mut self, scan_time: DateTime<Utc>) {
        self.set_created_at(scan_time);
        self.set_valid_from(scan_time);
    }

    /// Whether discovery is expected to keep this entity's `last_seen_at`
    /// advancing. Manually- and system-created entities are never re-observed
    /// by a scan, so their `last_seen_at` is frozen at creation and a freshness
    /// verdict on them would be meaningless — every hand-curated host would
    /// read as stale once it aged past the window.
    ///
    /// Defaults to `true` for the types that carry no `EntitySource` (Port,
    /// IPAddress, Interface, Binding — always created by discovery). Host,
    /// Service, Subnet and Vlan override it with `source.is_from_discovery()`.
    fn is_discovery_managed(&self) -> bool {
        true
    }

    /// Freshness of this entity as of `cutoff` — the instant before which
    /// `last_seen_at` counts as stale, from `Site::stale_cutoff`.
    ///
    /// The single definition of staleness. Both the digest and the read path
    /// call this so they cannot drift apart; the equivalent SQL predicate lives
    /// in `StorableFilter::stale_by_site` and must be kept in step with it.
    fn freshness(&self, cutoff: DateTime<Utc>) -> EntityFreshness {
        if !self.is_discovery_managed() || self.last_seen_at() >= cutoff {
            EntityFreshness::Current
        } else {
            EntityFreshness::Stale
        }
    }

    /// Returns a filter for "which of my rows did this discovery scan?"
    /// Top-level entities (Host/Subnet/Vlan) filter by `id IN
    /// scanned.<entity>_ids`; child entities (IPAddress/Port/Service/
    /// Interface/Binding) and the SubnetVlan junction filter by their
    /// own `id IN scanned.<entity>_ids` lists. The daemon populates the
    /// list authoritatively (server tracks no derived sets).
    fn scanned_in_session_filter(
        scanned: &crate::server::daemons::r#impl::api::ScannedEntityIds,
    ) -> crate::server::shared::storage::filter::StorableFilter<Self>;
}

/// Per-entity-type live-id → closed-id mappings populated parents-first
/// during snapshot close-and-clone. Children consult these to rewrite their
/// FK columns so closed copies reference closed parents.
#[derive(Debug, Default, Clone)]
pub struct FkMaps {
    pub hosts: HashMap<Uuid, Uuid>,
    pub subnets: HashMap<Uuid, Uuid>,
    pub vlans: HashMap<Uuid, Uuid>,
    pub services: HashMap<Uuid, Uuid>,
    pub ip_addresses: HashMap<Uuid, Uuid>,
    pub ports: HashMap<Uuid, Uuid>,
    pub interfaces: HashMap<Uuid, Uuid>,
    /// Live-binding-id → closed-binding-id. Populated when bindings are
    /// cloned (BINDINGS comes before DEPENDENCY_MEMBERS in CLONE_ORDER) so
    /// `DependencyMemberRecord::remap_fks_for_clone` can rewrite the
    /// optional `binding_id` to point at the closed binding's id rather
    /// than the live binding (whose state has moved on after `valid_from`
    /// advances).
    pub bindings: HashMap<Uuid, Uuid>,
    pub dependencies: HashMap<Uuid, Uuid>,
}

impl FkMaps {
    /// Lookup helper for entity_tags. The row's `entity_type` is typed as
    /// `EntityDiscriminants` in the app (serialized to/from text in DB via
    /// `SqlValue::EntityDiscriminant`). Returns None for org-scoped variants
    /// (Daemon, User, DaemonApiKey, UserApiKey, etc.) — those rows aren't
    /// cloned at site snapshot.
    pub fn lookup_by_entity_type(
        &self,
        entity_type: EntityDiscriminants,
        live_id: Uuid,
    ) -> Option<Uuid> {
        match entity_type {
            EntityDiscriminants::Host => self.hosts.get(&live_id).copied(),
            EntityDiscriminants::Service => self.services.get(&live_id).copied(),
            EntityDiscriminants::Subnet => self.subnets.get(&live_id).copied(),
            EntityDiscriminants::Dependency => self.dependencies.get(&live_id).copied(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod fk_maps_tests {
    use super::*;

    #[test]
    fn lookup_returns_closed_id_for_site_scoped_entity_types() {
        let mut maps = FkMaps::default();
        let live_host = Uuid::new_v4();
        let closed_host = Uuid::new_v4();
        let live_svc = Uuid::new_v4();
        let closed_svc = Uuid::new_v4();
        let live_subnet = Uuid::new_v4();
        let closed_subnet = Uuid::new_v4();
        let live_dep = Uuid::new_v4();
        let closed_dep = Uuid::new_v4();

        maps.hosts.insert(live_host, closed_host);
        maps.services.insert(live_svc, closed_svc);
        maps.subnets.insert(live_subnet, closed_subnet);
        maps.dependencies.insert(live_dep, closed_dep);

        assert_eq!(
            maps.lookup_by_entity_type(EntityDiscriminants::Host, live_host),
            Some(closed_host)
        );
        assert_eq!(
            maps.lookup_by_entity_type(EntityDiscriminants::Service, live_svc),
            Some(closed_svc)
        );
        assert_eq!(
            maps.lookup_by_entity_type(EntityDiscriminants::Subnet, live_subnet),
            Some(closed_subnet)
        );
        assert_eq!(
            maps.lookup_by_entity_type(EntityDiscriminants::Dependency, live_dep),
            Some(closed_dep)
        );
    }

    #[test]
    fn lookup_returns_none_for_org_scoped_entity_types() {
        // Org-scoped variants (Daemon, User, …) aren't cloned at site
        // snapshot, so lookup never has an entry for them.
        let maps = FkMaps::default();
        let some_id = Uuid::new_v4();
        assert!(
            maps.lookup_by_entity_type(EntityDiscriminants::Daemon, some_id)
                .is_none()
        );
        assert!(
            maps.lookup_by_entity_type(EntityDiscriminants::User, some_id)
                .is_none()
        );
    }

    #[test]
    fn lookup_returns_none_for_unknown_live_id() {
        let mut maps = FkMaps::default();
        maps.hosts.insert(Uuid::new_v4(), Uuid::new_v4());
        assert!(
            maps.lookup_by_entity_type(EntityDiscriminants::Host, Uuid::new_v4())
                .is_none()
        );
    }

    #[test]
    fn fk_maps_includes_bindings_field_for_dep_member_remap() {
        // Regression for the gotcha #1 in STATE.md / plan: dep_member's
        // optional binding_id remap requires `bindings` to be a sub-map.
        let mut maps = FkMaps::default();
        let live = Uuid::new_v4();
        let closed = Uuid::new_v4();
        maps.bindings.insert(live, closed);
        assert_eq!(maps.bindings.get(&live).copied(), Some(closed));
    }
}

#[cfg(test)]
mod discovery_tracked_stamping_tests {
    use chrono::TimeZone;
    use uuid::Uuid;

    use super::DiscoveryTracked;
    use crate::server::hosts::r#impl::base::{Host, HostBase};
    use crate::server::hosts::r#impl::name::HostNameSources;

    fn fresh_host() -> Host {
        Host::new(HostBase {
            name: crate::server::hosts::r#impl::name::HostName::manual("test".to_string()),
            site_id: Uuid::new_v4(),
            ..Default::default()
        })
    }

    #[test]
    fn refresh_scan_timestamps_only_touches_seen_and_updated() {
        let mut h = fresh_host();
        let original_created = h.created_at;
        let original_valid_from = h.valid_from;
        let scan_time = chrono::Utc.with_ymd_and_hms(2030, 1, 1, 12, 0, 0).unwrap();

        h.refresh_scan_timestamps(scan_time);

        assert_eq!(h.last_seen_at, scan_time, "last_seen_at refreshed");
        assert_eq!(h.updated_at, scan_time, "updated_at refreshed");
        assert_eq!(
            h.created_at, original_created,
            "created_at preserved (origin field)"
        );
        assert_eq!(
            h.valid_from, original_valid_from,
            "valid_from preserved (origin field)"
        );
    }

    #[test]
    fn originate_scan_timestamps_only_touches_created_and_valid_from() {
        let mut h = fresh_host();
        let original_seen = h.last_seen_at;
        let original_updated = h.updated_at;
        let scan_time = chrono::Utc.with_ymd_and_hms(2030, 1, 1, 12, 0, 0).unwrap();

        h.originate_scan_timestamps(scan_time);

        assert_eq!(h.created_at, scan_time, "created_at set");
        assert_eq!(h.valid_from, scan_time, "valid_from set");
        assert_eq!(
            h.last_seen_at, original_seen,
            "last_seen_at preserved (refresh field)"
        );
        assert_eq!(
            h.updated_at, original_updated,
            "updated_at preserved (refresh field)"
        );
    }

    #[test]
    fn refresh_then_originate_yields_all_four_at_scan_time() {
        let mut h = fresh_host();
        let scan_time = chrono::Utc.with_ymd_and_hms(2030, 1, 1, 12, 0, 0).unwrap();

        h.refresh_scan_timestamps(scan_time);
        h.originate_scan_timestamps(scan_time);

        assert_eq!(h.created_at, scan_time);
        assert_eq!(h.updated_at, scan_time);
        assert_eq!(h.valid_from, scan_time);
        assert_eq!(h.last_seen_at, scan_time);
    }
}

#[cfg(test)]
mod snapshotable_tests {
    use super::*;
    use crate::server::tags::entity_tags::{EntityTag, EntityTagBase};

    #[test]
    fn make_closed_copy_sets_lineage_to_live_id() {
        let live = EntityTag::new(EntityTagBase::new(
            Uuid::new_v4(),
            EntityDiscriminants::Host,
            Uuid::new_v4(),
        ));
        let live_id = live.id_value();

        let closed_at = chrono::Utc::now();
        let closed = live.make_closed_copy(closed_at);

        assert_eq!(closed.lineage_id(), Some(live_id));
        assert_eq!(closed.valid_to(), Some(closed_at));
        // valid_from is preserved — closed row covers [valid_from, close_at]
        assert_eq!(closed.valid_from(), live.valid_from());
        // The id is unchanged on the copy itself; the SnapshotService
        // assigns a new uuid via set_id_value before INSERT.
        assert_eq!(closed.id_value(), live_id);
    }

    #[test]
    fn make_closed_copy_preserves_payload_fields() {
        let entity_id = Uuid::new_v4();
        let tag_id = Uuid::new_v4();
        let live = EntityTag::new(EntityTagBase::new(
            entity_id,
            EntityDiscriminants::Service,
            tag_id,
        ));
        let closed = live.make_closed_copy(chrono::Utc::now());
        assert_eq!(closed.base.entity_id, entity_id);
        assert_eq!(closed.base.entity_type, EntityDiscriminants::Service);
        assert_eq!(closed.base.tag_id, tag_id);
    }
}
