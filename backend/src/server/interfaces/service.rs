use anyhow::Result;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use uuid::Uuid;
use validator::ValidationError;

use crate::server::ip_addresses::r#impl::base::mac_of;
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    interface_neighbors::service::InterfaceNeighborService,
    interfaces::r#impl::base::{Interface, InterfaceDataComplete},
    ip_addresses::service::IPAddressService,
    shared::{
        events::bus::EventBus,
        services::traits::{ChildCrudService, CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Entity, Storage},
        },
    },
    tags::entity_tags::EntityTagService,
};

pub struct InterfaceService {
    storage: Arc<GenericPostgresStorage<Interface>>,
    event_bus: Arc<EventBus>,
    ip_address_service: Arc<IPAddressService>,
    interface_neighbor_service: Arc<InterfaceNeighborService>,
}

impl EventBusService<Interface> for InterfaceService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, entity: &Interface) -> Option<Uuid> {
        Some(entity.base.network_id)
    }

    fn get_organization_id(&self, _entity: &Interface) -> Option<Uuid> {
        None
    }
}

impl CrudService<Interface> for InterfaceService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Interface>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl ChildCrudService<Interface> for InterfaceService {}

impl InterfaceService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<Interface>>,
        event_bus: Arc<EventBus>,
        ip_address_service: Arc<IPAddressService>,
        interface_neighbor_service: Arc<InterfaceNeighborService>,
    ) -> Self {
        Self {
            storage,
            event_bus,
            ip_address_service,
            interface_neighbor_service,
        }
    }

    /// Get all if entries for a specific host, ordered by ifIndex.
    /// SCD2: only live rows.
    pub async fn get_for_host(&self, host_id: &Uuid) -> Result<Vec<Interface>> {
        let filter = StorableFilter::<Interface>::new_from_host_ids(&[*host_id]).live();
        self.storage.get_all_ordered(filter, "if_index ASC").await
    }

    /// Get if entries for multiple hosts, ordered by ifIndex within each host.
    /// `at = None` reads live rows; `Some(t)` reads SCD2 state as of `t`
    /// (snapshot-view hydration).
    pub async fn get_for_hosts(
        &self,
        host_ids: &[Uuid],
        at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<HashMap<Uuid, Vec<Interface>>> {
        if host_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = StorableFilter::<Interface>::new_from_host_ids(host_ids).live_or_as_of(at);
        let entries = self
            .storage
            .get_all_ordered(filter, "host_id ASC, if_index ASC")
            .await?;

        let mut result: HashMap<Uuid, Vec<Interface>> = HashMap::new();
        for entry in entries {
            result.entry(entry.base.host_id).or_default().push(entry);
        }
        Ok(result)
    }

    /// Validate FK relationships for an Interface.
    ///
    /// Validates:
    /// - ip_address_id must reference an Interface on the same host
    /// - If both Interface and Interface have MAC addresses, they should match
    ///
    /// Neighbour relationships moved off `Interface` in GH #701 (resolved rows live in
    /// `interface_neighbor_interfaces`/`interface_neighbor_hosts`, upserted only by
    /// `HostService`'s resolution ladder, never by a user-facing write) — there is no longer a
    /// `neighbor` field on this type to validate here.
    pub async fn validate_relationships(&self, entry: &Interface) -> Result<()> {
        // ip_address_id: must be on SAME host, and MAC addresses should match if both present
        if let Some(ip_address_id) = entry.base.ip_address_id {
            let ip_address = self
                .ip_address_service
                .get_by_id(&ip_address_id)
                .await?
                .ok_or_else(|| {
                    ValidationError::new("ip_address_id references a non-existent Interface")
                })?;

            if ip_address.base.host_id != entry.base.host_id {
                return Err(ValidationError::new(
                    "ip_address_id must reference an Interface on the same host",
                )
                .into());
            }

            // Validate MAC address consistency if both have MAC addresses
            if let (Some(if_entry_mac), Some(ip_address_mac)) =
                (&entry.base.mac_address, &ip_address.base.mac_address)
                && if_entry_mac != ip_address_mac
            {
                return Err(ValidationError::new(
                    "ip_address_id references an Interface with a different MAC address",
                )
                .into());
            }
        }

        Ok(())
    }

    /// Lookup interfaces whose ip_address_id is in the given set.
    /// Used by subnet_vlans reconciliation to aggregate native_vlan_id observations.
    pub async fn get_by_ip_address_ids(&self, ids: &[Uuid]) -> Result<Vec<Interface>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        // SCD2: only live rows.
        let filter =
            StorableFilter::<Interface>::new_from_uuids_column("ip_address_id", ids).live();
        self.storage.get_all(filter).await
    }

    /// Create or update an interface during discovery, matching on a tiered identity:
    ///
    /// 1. `(host_id, if_name)` when incoming `if_name.is_some()` — strong identifier
    ///    from ifXTable, survives reboots/config reloads
    /// 2. `(host_id, if_index)` — fallback for legacy devices without ifXTable, and
    ///    for pre-existing rows written before the `if_name` column was added (first
    ///    post-upgrade rescan finds those rows by `if_index` and writes `if_name`,
    ///    after which tier 1 owns the match)
    /// 3. `(host_id, mac_address)` with single-MAC guard — last resort for ports
    ///    that got both renamed and renumbered but kept their NIC
    ///
    /// On match, preserves id + created_at + mac_address + the ifTable-shaped fields (if_name,
    /// if_index, if_type, admin/oper status, if_descr) wherever the incoming payload has none of
    /// its own — via `preserve_immutable_fields` — and overwrites the rest with the incoming
    /// payload. Skips relationship validation (data from trusted SNMP source).
    ///
    /// `claimed` holds the ids of existing rows already matched (or created) by
    /// earlier interfaces in the *same* discovery batch. A row may be claimed by
    /// at most one incoming interface per scan — without this, a host that
    /// reports many ifTable entries sharing one weak identity (e.g. an L2 switch
    /// whose IP-less ports all carry the chassis MAC and no ifName) would see
    /// every port re-match and overwrite the first row via tier-3, collapsing the
    /// whole ifTable onto a single interface (issue #614).
    pub async fn create_or_update_from_discovery(
        &self,
        entry: Interface,
        claimed: &HashSet<Uuid>,
        collected: InterfaceDataComplete,
        authentication: AuthenticatedEntity,
    ) -> Result<Interface> {
        let mut entry = entry;
        entry.normalize_blank_identity();
        // A pre-#701 daemon's scalar LLDP/CDP submission has already been folded in here by
        // `DiscoveryInterface` at the API boundary, so this path sees one shape only.
        // Captured before `entry` moves into `create`/`update` below — the candidates this scan
        // submitted for this port, independent of which branch persists the interface itself.
        let submitted_candidates = std::mem::take(&mut entry.base.neighbor_candidates);

        let existing = self.find_matching_existing(&entry, claimed).await?;

        let persisted = if let Some(existing_entry) = existing {
            let mut updated = entry;
            updated.id = existing_entry.id;
            updated.preserve_immutable_fields(&existing_entry);
            updated.preserve_uncollected_data(&existing_entry, collected);
            self.update(&mut updated, authentication).await?
        } else {
            // SCD2 origin: no match found, this is a new insert. Stamp
            // created_at + valid_from to the entity's already-refreshed
            // `last_seen_at` (set by the discovery handler) so all four
            // temporal columns line up at one canonical scan_time.
            use crate::server::shared::storage::snapshot::DiscoveryTracked;
            let mut entry = entry;
            entry.originate_scan_timestamps(entry.last_seen_at);
            self.create(entry, authentication).await?
        };

        // Replace this port's candidate rows now that it has a real, persisted id. Per-group
        // completeness (a walk cut short by timeout) is honored inside the service the same way
        // `preserve_uncollected_data` above honors it for `fdb_macs`/VLAN membership.
        self.interface_neighbor_service
            .replace_candidates_from_discovery(
                persisted.base.network_id,
                persisted.id,
                submitted_candidates,
                collected,
            )
            .await?;

        Ok(persisted)
    }

    /// Tiered lookup: if_name → if_index → mac_address with single-MAC guard.
    /// Loads the host's live interfaces once, then delegates the decision to the
    /// pure [`match_existing_interface`] so the tier logic stays unit-testable.
    async fn find_matching_existing(
        &self,
        entry: &Interface,
        claimed: &HashSet<Uuid>,
    ) -> Result<Option<Interface>> {
        let host_id = entry.base.host_id;

        // Load host's interfaces once for all three tiers. `claimed` excludes rows
        // already matched/created earlier in this batch so siblings sharing a weak
        // identity (chassis MAC, NULL if_name) can't collapse onto one row.
        let existing = self.get_for_host(&host_id).await?;

        let matched_id = match_existing_interface(entry, &existing, claimed);

        Ok(matched_id.and_then(|id| existing.into_iter().find(|e| e.id == id)))
    }
}

/// Pure tiered identity match used by discovery dedup.
///
/// Returns the id of the existing row the incoming `entry` should update, or
/// `None` to insert a new row. Tiers, in order:
/// 1. `(host_id, if_name)` when incoming `if_name` is present — strongest, from
///    ifXTable; survives reboots/config reloads.
/// 2. `(host_id, if_index)` — legacy devices without ifXTable and pre-`if_name`
///    rows.
/// 3. `(host_id, mac_address)` with single-MAC guard — last resort for a port
///    that was both renamed and renumbered but kept its NIC.
///
/// Any row whose id is in `claimed` (already matched/created by an earlier
/// interface in the same batch) is skipped at every tier, so two distinct
/// ifTable entries can never collapse onto one row (issue #614). The single-MAC
/// guard is applied after the `claimed` filter: a MAC shared by more than one
/// *unclaimed* existing row stays ambiguous (VLAN sub-interfaces / bond members)
/// and yields no match.
pub(crate) fn match_existing_interface(
    entry: &Interface,
    existing: &[Interface],
    claimed: &HashSet<Uuid>,
) -> Option<Uuid> {
    let available = |e: &&Interface| !claimed.contains(&e.id);

    // Tier 1: (host_id, if_name) — strong identifier when present
    if let Some(ref if_name) = entry.base.if_name
        && let Some(found) = existing
            .iter()
            .filter(available)
            .find(|e| e.base.if_name.as_deref() == Some(if_name.as_str()))
    {
        return Some(found.id);
    }

    // Tier 2: (host_id, if_index), and only for a row that has one.
    //
    // The `Some` matters more than it looks. `if_index` is nullable — a port learned from a
    // neighbour's advertisement has none — and a bare `==` would make `None == None` true, so
    // every such port on a host would match the first other index-less row and overwrite it.
    // That is GH #614's collapse (a whole ifTable folding onto one row) reintroduced by a
    // comparison that still compiles.
    if let Some(if_index) = entry.base.if_index
        && let Some(found) = existing
            .iter()
            .filter(available)
            .find(|e| e.base.if_index == Some(if_index))
    {
        return Some(found.id);
    }

    // Tier 3: (host_id, mac_address) with single-MAC guard. A MAC shared by
    // multiple unclaimed existing rows indicates VLAN sub-interfaces or bond
    // members and must not collapse into a single match — only accept a 1:1
    // MAC pairing.
    if let Some(mac) = mac_of(&entry.base.mac_address) {
        let mut candidates = existing
            .iter()
            .filter(available)
            .filter(|e| mac_of(&e.base.mac_address) == Some(mac));
        if let Some(first) = candidates.next()
            && candidates.next().is_none()
        {
            return Some(first.id);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
    use crate::server::services::r#impl::patterns::ClientProbe;
    use crate::server::shared::attribution::AttributeSource;

    /// What a credentialed SNMP walk claims for a MAC it read off the device itself.
    const SNMP_MAC: AttributeSource = AttributeSource::Probe(ClientProbe::Snmp);
    use crate::server::interfaces::r#impl::base::InterfaceBase;
    use mac_address::MacAddress;

    fn make_iface(if_index: i32, if_name: Option<&str>, mac: Option<&str>) -> Interface {
        let mut entry = make_indexless_iface(if_name, mac);
        entry.base.if_index = Some(if_index);
        entry.base.if_descr = Some(format!("ifIndex {if_index}"));
        entry
    }

    /// A port with no ifIndex — the shape a neighbour advertisement produces, where the far end
    /// names its port and nothing has read that device's ifTable.
    fn make_indexless_iface(if_name: Option<&str>, mac: Option<&str>) -> Interface {
        Interface::new(InterfaceBase {
            host_id: Uuid::nil(),
            if_descr: if_name.map(str::to_string),
            if_name: if_name.map(String::from),
            mac_address: mac
                .map(|s| s.parse::<MacAddress>().unwrap())
                .map(|m| MacEvidence::new(MacEvidenceValue(m), SNMP_MAC)),
            ..Default::default()
        })
    }

    /// Replay a discovery batch through the real tiered matcher, mirroring
    /// `create_or_update_from_discovery` + the `create_with_children` loop:
    /// a `None` match inserts a new row, a `Some(id)` match overwrites that row
    /// in place (preserving immutable fields, exactly like the update path).
    /// When `batch_aware` is false the `claimed` set is never consulted,
    /// reproducing the pre-fix behaviour.
    fn run_batch_from(
        mut persisted: Vec<Interface>,
        incoming: Vec<Interface>,
        batch_aware: bool,
    ) -> Vec<Interface> {
        let mut claimed: HashSet<Uuid> = HashSet::new();
        let empty: HashSet<Uuid> = HashSet::new();
        for mut entry in incoming {
            entry.normalize_blank_identity();
            let claim_set = if batch_aware { &claimed } else { &empty };
            match match_existing_interface(&entry, &persisted, claim_set) {
                Some(id) => {
                    let pos = persisted.iter().position(|e| e.id == id).unwrap();
                    let mut updated = entry;
                    updated.id = id;
                    updated.preserve_immutable_fields(&persisted[pos]);
                    persisted[pos] = updated;
                    claimed.insert(id);
                }
                None => {
                    claimed.insert(entry.id);
                    persisted.push(entry);
                }
            }
        }
        persisted
    }

    fn omada_iftable() -> Vec<Interface> {
        // ifIndex 1: management interface (has ifName + chassis MAC + an IP).
        // ifIndex 49153..=49168: 16 physical switch ports — no ifName (device
        // omits ifXTable) and all reporting the same chassis ifPhysAddress, no IP.
        let chassis_mac = "00:11:22:33:44:55";
        let mut ifaces = vec![make_iface(1, Some("Vlan-interface1"), Some(chassis_mac))];
        for if_index in 49153..=49168 {
            ifaces.push(make_iface(if_index, None, Some(chassis_mac)));
        }
        assert_eq!(ifaces.len(), 17);
        ifaces
    }

    /// Issue #614 characterization: all 17 ifTable entries must persist as
    /// distinct rows on first discovery.
    #[test]
    fn omada_switch_all_17_interfaces_persist() {
        let persisted = run_batch_from(Vec::new(), omada_iftable(), true);
        assert_eq!(persisted.len(), 17, "all 17 interfaces must persist");

        let mut indexes: Vec<Option<i32>> = persisted.iter().map(|e| e.base.if_index).collect();
        indexes.sort();
        let mut expected: Vec<Option<i32>> =
            std::iter::once(1).chain(49153..=49168).map(Some).collect();
        expected.sort();
        assert_eq!(indexes, expected, "every ifIndex must be represented once");
    }

    /// Regression witness: WITHOUT the batch-aware `claimed` exclusion the 16
    /// IP-less ports collapse via tier-3 onto the management interface (which
    /// keeps its original ifName via `preserve_immutable_fields`), leaving a
    /// single row — the exact "17 collected → 1 persisted" defect.
    #[test]
    fn pre_fix_behaviour_collapses_to_one() {
        let collapsed = run_batch_from(Vec::new(), omada_iftable(), false);
        assert_eq!(collapsed.len(), 1, "demonstrates the issue #614 collapse");
        assert_eq!(
            collapsed[0].base.if_name.as_deref(),
            Some("Vlan-interface1"),
            "the lone survivor retains the management interface's name"
        );
    }

    /// Some switches answer ifXTable `ifName` with a zero-length string on every port. That is
    /// "this device has no name for this port", but it arrives as `Some("")` and would otherwise
    /// be treated as a real, shared name: every such port claims the same tier-1 identity, so a
    /// rescan pairs incoming ports with whichever blank-named row happens to be unclaimed rather
    /// than with their own. (In Postgres the same false identity collides with the partial unique
    /// index on `(host_id, if_name)`.) Blank is absence, so each port falls through to its
    /// ifIndex.
    #[test]
    fn blank_if_names_do_not_become_a_shared_identity() {
        let chassis_mac = "00:11:22:33:44:55";
        let blank_named: Vec<Interface> = (49153..=49156)
            .map(|if_index| make_iface(if_index, Some(""), Some(chassis_mac)))
            .collect();

        let first = run_batch_from(Vec::new(), blank_named.clone(), true);
        assert_eq!(first.len(), 4, "every port must persist as its own row");

        // Rescan in a different order than the first walk — a device is under no obligation to
        // enumerate its ifTable identically, and identity must not depend on ordering.
        let mut reordered = blank_named;
        reordered.reverse();
        let second = run_batch_from(first.clone(), reordered, true);

        assert_eq!(second.len(), 4, "rescan must not create duplicate rows");
        for iface in &second {
            let original = first
                .iter()
                .find(|e| e.base.if_index == iface.base.if_index)
                .expect("every ifIndex is still represented");
            assert_eq!(
                iface.id, original.id,
                "ifIndex {:?} must keep its own row across a rescan",
                iface.base.if_index
            );
        }
    }

    /// Re-scanning the same device updates the existing 17 rows in place rather
    /// than duplicating or collapsing them — tier-2 (if_index) handles the
    /// nameless ports, tier-1 (if_name) the management interface.
    #[test]
    fn rescan_updates_existing_rows_without_duplication() {
        let first = run_batch_from(Vec::new(), omada_iftable(), true);
        assert_eq!(first.len(), 17);

        let second = run_batch_from(first.clone(), omada_iftable(), true);
        assert_eq!(second.len(), 17, "re-scan must not create duplicates");

        let first_ids: HashSet<Uuid> = first.iter().map(|e| e.id).collect();
        let second_ids: HashSet<Uuid> = second.iter().map(|e| e.id).collect();
        assert_eq!(first_ids, second_ids, "re-scan must reuse the same row ids");
    }

    /// Two existing rows sharing one MAC (VLAN sub-interfaces / bond members)
    /// stay ambiguous: the single-MAC guard refuses to match, so a nameless
    /// incoming port with that MAC and a new if_index inserts rather than
    /// hijacking either sibling.
    #[test]
    fn shared_mac_across_multiple_rows_does_not_match() {
        let mac = "aa:bb:cc:dd:ee:ff";
        let persisted = vec![
            make_iface(10, Some("bond0.10"), Some(mac)),
            make_iface(11, Some("bond0.11"), Some(mac)),
        ];
        let incoming = make_iface(12, None, Some(mac));
        assert_eq!(
            match_existing_interface(&incoming, &persisted, &HashSet::new()),
            None,
            "ambiguous shared MAC must not collapse onto a sibling"
        );
    }

    /// Two ports that never had an ifIndex read do not collapse onto each other.
    ///
    /// The index tier compares `entry.base.if_index` against each candidate's, and `if_index` is
    /// nullable — a port learned from an LLDP or CDP advertisement has none. A bare `==` makes
    /// `None == None` true, so the first index-less row on a host would swallow every other one:
    /// GH #614's collapse (a whole ifTable folding onto a single row) reintroduced by a
    /// comparison that still compiles and still passes every existing test.
    ///
    /// The names differ, so the name tier cannot save this — reaching the index tier at all is
    /// the bug.
    #[test]
    fn ports_with_no_if_index_do_not_collapse_onto_one_another() {
        // Both already stored and neither claimed by this batch, so the newcomer reaches the index
        // tier with unclaimed index-less rows in front of it — which is the only way the defect
        // bites, and the reason a batch whose names all match cannot expose it.
        let persisted = vec![
            make_indexless_iface(Some("Ten-GigabitEthernet1/0/1"), None),
            make_indexless_iface(Some("GigabitEthernet1/0/8"), None),
        ];
        let incoming = vec![make_indexless_iface(Some("eth0"), None)];

        let persisted = run_batch_from(persisted, incoming, true);

        let mut names: Vec<&str> = persisted
            .iter()
            .filter_map(|e| e.base.if_name.as_deref())
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            vec!["GigabitEthernet1/0/8", "Ten-GigabitEthernet1/0/1", "eth0"],
            "a third advertised port must not overwrite the first index-less row"
        );
    }

    /// An advertised port is upgraded in place when the device is finally walked, rather than
    /// duplicated beside the real ifTable row.
    ///
    /// This is why the minted row is keyed on the name the far end published: an SNMP walk of that
    /// device returns the same string as `ifName`, so the name tier matches and the row gains its
    /// ifIndex, type and statuses instead of the host ending up with two ports called `eth0`.
    #[test]
    fn a_real_walk_upgrades_the_port_a_neighbour_advertised() {
        let persisted = vec![make_indexless_iface(Some("eth0"), None)];
        let inferred_id = persisted[0].id;

        let persisted = run_batch_from(persisted, vec![make_iface(3, Some("eth0"), None)], true);

        assert_eq!(persisted.len(), 1, "the walk must not add a second eth0");
        assert_eq!(persisted[0].id, inferred_id, "same row, upgraded");
        assert_eq!(persisted[0].base.if_index, Some(3));
    }

    /// The acceptance bar for the PROFINET DCP item: "a host DCP also saw does not lose its
    /// SNMP-discovered interfaces." A DCP submission carries no `if_name`/`if_index` at all — it
    /// has neither, unlike the LLDP-minted shape `make_indexless_iface` was written for, which at
    /// least has a name — so Tiers 1 and 2 both skip and only Tier 3 (MAC) can place it. If it
    /// matched nothing it would insert as a *second* row for the same physical port; if the
    /// existing row's Tier 1/2 identity were lost in the merge that would be its own bug. Neither
    /// happens: the incoming MAC-only entry resolves onto the existing SNMP row, which keeps its
    /// name and index.
    #[test]
    fn a_mac_only_dcp_submission_matches_the_snmp_row_sharing_its_mac_rather_than_duplicating_it() {
        let mac = "aa:bb:cc:dd:ee:ff";
        let snmp_row = make_iface(7, Some("Gi1/0/7"), Some(mac));
        let existing_id = snmp_row.id;

        let dcp_entry = make_indexless_iface(None, Some(mac));
        let persisted = run_batch_from(vec![snmp_row], vec![dcp_entry], true);

        assert_eq!(
            persisted.len(),
            1,
            "must resolve onto the existing row, not add a second one"
        );
        assert_eq!(persisted[0].id, existing_id);
        assert_eq!(
            persisted[0].base.if_name.as_deref(),
            Some("Gi1/0/7"),
            "the SNMP row's identity must survive the merge"
        );
        assert_eq!(persisted[0].base.if_index, Some(7));
    }
}
