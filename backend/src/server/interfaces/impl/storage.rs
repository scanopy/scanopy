use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::ip_addresses::r#impl::base::MacEvidenceValue;
use crate::server::shared::attribution::{AttributeSource, Attributed};
use crate::server::shared::storage::attributed;
use crate::server::{
    interfaces::r#impl::base::{
        FdbMacsValue, IfAdminStatus, IfOperStatus, Interface, InterfaceBase,
    },
    shared::{
        entities::EntityDiscriminants,
        product_area::ProductArea,
        storage::{
            child::ChildStorableEntity,
            snapshot::{DiscoveryTracked, FkMaps, Snapshotable},
            traits::{Entity, SqlValue, Storable},
        },
    },
};

impl Storable for Interface {
    type BaseData = InterfaceBase;

    fn table_name() -> &'static str {
        "interfaces"
    }

    const HAS_SCD2: bool = true;

    fn is_live_row(&self) -> bool {
        self.valid_to.is_none()
    }

    fn new(base: Self::BaseData) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            valid_from: now,
            valid_to: None,
            lineage_id: None,
            last_seen_at: now,
            last_discovery_id: None,
            first_discovery_id: None,
            display_name: None,
            base,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            valid_from,
            valid_to,
            lineage_id,
            last_seen_at,
            last_discovery_id,
            first_discovery_id,
            display_name: _,
            base:
                Self::BaseData {
                    host_id,
                    site_id,
                    if_index,
                    if_descr,
                    if_name,
                    if_alias,
                    if_type,
                    speed_bps,
                    admin_status,
                    oper_status,
                    mac_address,
                    ip_address_id,
                    ip_configured,
                    // Wire-only (daemon submission), never a column on `interfaces` — drained into
                    // `interface_neighbor_candidates` by `InterfaceService::create_or_update_from_
                    // discovery` before this ever runs. See `interface_neighbors`.
                    neighbor_candidates: _,
                    fdb_macs,
                    native_vlan_id,
                    vlan_ids,
                    vlan_membership_source,
                    interface_source,
                },
        } = self.clone();

        let [mac_value, mac_source] = attributed::optional_params(&mac_address);
        let [fdb_value, fdb_source] = attributed::optional_params(&fdb_macs);

        let columns = vec![
            "id",
            "host_id",
            "site_id",
            "if_index",
            "if_descr",
            "if_name",
            "if_alias",
            "if_type",
            "speed_bps",
            "admin_status",
            "oper_status",
            "mac_address",
            "mac_address_source",
            "ip_address_id",
            "ip_configured",
            "fdb_macs",
            "fdb_macs_source",
            "native_vlan_id",
            "vlan_ids",
            "vlan_membership_source",
            "interface_source",
            "created_at",
            "updated_at",
            "valid_from",
            "valid_to",
            "lineage_id",
            "last_seen_at",
            "last_discovery_id",
            "first_discovery_id",
        ];

        let values = vec![
            SqlValue::Uuid(id),
            SqlValue::Uuid(host_id),
            SqlValue::Uuid(site_id),
            SqlValue::OptionalI32(if_index),
            SqlValue::OptionalString(if_descr),
            SqlValue::OptionalString(if_name),
            SqlValue::OptionalString(if_alias),
            SqlValue::OptionalI32(if_type),
            SqlValue::OptionalI64(speed_bps),
            SqlValue::OptionalI32(admin_status.map(i32::from)),
            SqlValue::OptionalI32(oper_status.map(i32::from)),
            mac_value,
            mac_source,
            SqlValue::OptionalUuid(ip_address_id),
            SqlValue::Bool(ip_configured),
            fdb_value,
            fdb_source,
            SqlValue::OptionalUuid(native_vlan_id),
            SqlValue::OptionVecUuid(vlan_ids),
            SqlValue::AttributeSource(vlan_membership_source),
            SqlValue::AttributeSource(interface_source),
            SqlValue::Timestamp(created_at),
            SqlValue::Timestamp(updated_at),
            SqlValue::Timestamp(valid_from),
            SqlValue::OptionTimestamp(valid_to),
            SqlValue::OptionalUuid(lineage_id),
            SqlValue::Timestamp(last_seen_at),
            SqlValue::OptionalUuid(last_discovery_id),
            SqlValue::OptionalUuid(first_discovery_id),
        ];

        Ok((columns, values))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        // Read as `Option` because the columns are nullable: a port learned from a neighbour's
        // advertisement carries no status, and `row.get::<i32>` *panics* on NULL rather than
        // returning an error, so a non-optional read here would take the request down.
        let admin_status_raw: Option<i32> = row.get("admin_status");
        let oper_status_raw: Option<i32> = row.get("oper_status");

        // Handle speed_bps which might be NULL or a large value
        let speed_bps: Option<i64> = row.get("speed_bps");

        // Read mac_address from MACADDR column
        let mac_address = attributed::read_optional::<MacEvidenceValue>(row)?;

        let mut interface = Interface {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            last_seen_at: row.get("last_seen_at"),
            last_discovery_id: row.get("last_discovery_id"),
            first_discovery_id: row.get("first_discovery_id"),
            // Never stored: computed below from the row, so every endpoint that returns an
            // interface names it the same way.
            display_name: None,
            base: InterfaceBase {
                host_id: row.get("host_id"),
                site_id: row.get("site_id"),
                if_index: row.get("if_index"),
                if_descr: row.get("if_descr"),
                if_name: row.get("if_name"),
                if_alias: row.get("if_alias"),
                if_type: row.get("if_type"),
                speed_bps,
                admin_status: admin_status_raw.map(IfAdminStatus::from),
                oper_status: oper_status_raw.map(IfOperStatus::from),
                mac_address,
                ip_address_id: row.get("ip_address_id"),
                ip_configured: row.get("ip_configured"),
                neighbor_candidates: Vec::new(),
                fdb_macs: attributed::read_optional::<FdbMacsValue>(row)?,
                native_vlan_id: row.get("native_vlan_id"),
                vlan_ids: row
                    .try_get::<Option<serde_json::Value>, _>("vlan_ids")
                    .ok()
                    .flatten()
                    .and_then(|v| serde_json::from_value(v).ok()),
                vlan_membership_source: attributed::read_source(row, "vlan_membership_source")?,
                interface_source: attributed::read_source(row, "interface_source")?,
            },
        };
        interface.display_name = Some(interface.display_name());
        Ok(interface)
    }
}

impl Entity for Interface {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    fn set_created_at(&mut self, time: DateTime<Utc>) {
        self.created_at = time;
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Interface
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Interface";
    const ENTITY_NAME_PLURAL: &'static str = "Interfaces";
    const ENTITY_DESCRIPTION: &'static str =
        "SNMP ifTable entries. Physical and logical interfaces discovered via SNMP on hosts.";

    fn product_area() -> ProductArea {
        ProductArea::Inventory
    }

    fn site_id(&self) -> Option<Uuid> {
        Some(self.base.site_id)
    }

    fn organization_id(&self) -> Option<Uuid> {
        None
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }
}

impl Interface {
    /// The MAC merges by rung rather than being pinned to whatever wrote it first. Pinning meant
    /// a MAC a router's ARP cache reported could never be corrected by the device answering for
    /// itself — first-write-wins under another name, and the same failure the host attributes
    /// had. `MacEvidenceValue` is not refreshable, so an equal-rung re-read still cannot move it.
    pub(crate) fn merge_mac_address(&mut self, existing: &Self) {
        let mut merged = existing.base.mac_address.clone();
        if let Some(incoming) = self.base.mac_address.clone() {
            Attributed::apply(&mut merged, incoming);
        }
        self.base.mac_address = merged;
    }

    pub(crate) fn merge_interface_source(&mut self, existing: &Self) {
        // A port the forwarding-table pass recorded becomes the device's own once a scan of the
        // device reports it, so the walk's source replaces the inferred one rather than yielding.
        self.base.interface_source =
            match (existing.base.interface_source, self.base.interface_source) {
                (AttributeSource::ForwardingTable, incoming)
                    if incoming != AttributeSource::Unspecified =>
                {
                    incoming
                }
                (existing, incoming) => existing.keep_or_heal(incoming),
            };
    }
}

impl ChildStorableEntity for Interface {
    fn parent_column() -> &'static str {
        "host_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.host_id
    }
}

impl Snapshotable for Interface {
    fn id_value(&self) -> Uuid {
        self.id
    }
    fn set_id_value(&mut self, id: Uuid) {
        self.id = id;
    }
    fn valid_from(&self) -> DateTime<Utc> {
        self.valid_from
    }
    fn valid_to(&self) -> Option<DateTime<Utc>> {
        self.valid_to
    }
    fn lineage_id(&self) -> Option<Uuid> {
        self.lineage_id
    }
    fn set_valid_from(&mut self, t: DateTime<Utc>) {
        self.valid_from = t;
    }
    fn set_valid_to(&mut self, t: Option<DateTime<Utc>>) {
        self.valid_to = t;
    }
    fn set_lineage_id(&mut self, id: Option<Uuid>) {
        self.lineage_id = id;
    }

    /// GH #701 removed `neighbor` from this struct entirely, so `interfaces` no longer carries a
    /// self-reference or an `Interface->Host` FK — both moved to `interface_neighbor_interfaces`/
    /// `interface_neighbor_hosts`, which remap against `maps.interfaces`/`maps.hosts` directly in
    /// their own `remap_fks_for_clone` (neither is a self-reference from where it lives now, so
    /// neither needs the `own_clone_ref` two-pass dance `Interface` used to require). Only the
    /// FKs that stayed on this table remain here.
    fn remap_fks_for_clone(&mut self, maps: &FkMaps) {
        if let Some(closed) = maps.hosts.get(&self.base.host_id) {
            self.base.host_id = *closed;
        }
        if let Some(ip_id) = self.base.ip_address_id
            && let Some(closed) = maps.ip_addresses.get(&ip_id)
        {
            self.base.ip_address_id = Some(*closed);
        }
        if let Some(vlan_id) = self.base.native_vlan_id
            && let Some(closed) = maps.vlans.get(&vlan_id)
        {
            self.base.native_vlan_id = Some(*closed);
        }
        // `vlan_ids` (JSONB array) stays as-is — a cross-host reference that may point outside
        // this site's snapshot; as-of joins handle resolution.
    }
}

impl DiscoveryTracked for Interface {
    fn last_seen_at(&self) -> DateTime<Utc> {
        self.last_seen_at
    }
    fn last_discovery_id(&self) -> Option<Uuid> {
        self.last_discovery_id
    }
    fn first_discovery_id(&self) -> Option<Uuid> {
        self.first_discovery_id
    }
    fn set_last_seen_at(&mut self, t: DateTime<Utc>) {
        self.last_seen_at = t;
    }
    fn set_last_discovery_id(&mut self, id: Option<Uuid>) {
        self.last_discovery_id = id;
    }
    fn set_first_discovery_id(&mut self, id: Option<Uuid>) {
        self.first_discovery_id = id;
    }

    fn scanned_in_session_filter(
        scanned: &crate::server::daemons::r#impl::api::ScannedEntityIds,
    ) -> crate::server::shared::storage::filter::StorableFilter<Self> {
        crate::server::shared::storage::filter::StorableFilter::<Self>::new_from_uuids_column(
            "id",
            &scanned.interface_ids,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::ip_addresses::r#impl::base::{MacEvidence, MacEvidenceValue};
    use crate::server::services::r#impl::patterns::ClientProbe;
    use crate::server::shared::attribution::AttributeSource;

    /// What a credentialed SNMP walk claims for a MAC it read off the device itself. Named once so
    /// a fixture cannot assert a provenance no real scan produces.
    const SNMP_MAC: AttributeSource = AttributeSource::Probe(ClientProbe::Snmp);

    use mac_address::MacAddress;

    fn make_interface(if_index: i32, if_name: Option<&str>, mac: Option<&str>) -> Interface {
        Interface::new(InterfaceBase {
            host_id: Uuid::new_v4(),
            site_id: Uuid::new_v4(),
            if_index: Some(if_index),
            if_name: if_name.map(String::from),
            mac_address: mac
                .map(|s| s.parse::<MacAddress>().unwrap())
                .map(|m| MacEvidence::new(MacEvidenceValue(m), SNMP_MAC)),
            ..Default::default()
        })
    }

    #[test]
    fn preserve_immutable_fields_keeps_existing_if_name_when_incoming_is_none() {
        let existing = make_interface(5, Some("GigabitEthernet0/1"), None);
        let mut incoming = make_interface(5, None, None);

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(
            incoming.base.if_name.as_deref(),
            Some("GigabitEthernet0/1"),
            "Existing if_name must survive a scan that dropped it; otherwise tier-1 matching silently breaks next time."
        );
    }

    /// A port the forwarding-table pass recorded becomes the device's own once the device's walk
    /// reports it: the walk's source replaces the inferred one. Any other source still keeps the
    /// first one recorded.
    #[test]
    fn a_walk_reporting_a_recorded_port_takes_over_its_source() {
        let mut existing = make_interface(1, None, Some("f8:bc:12:dc:14:01"));
        existing.base.interface_source = AttributeSource::ForwardingTable;
        let mut walked = make_interface(1, Some("eth0"), Some("f8:bc:12:dc:14:01"));
        walked.base.interface_source = SNMP_MAC;
        walked.preserve_immutable_fields(&existing);
        assert_eq!(walked.base.interface_source, SNMP_MAC);

        let mut kept = make_interface(1, None, None);
        kept.base.interface_source = AttributeSource::ArpReply;
        let mut later = make_interface(1, None, None);
        later.base.interface_source = SNMP_MAC;
        later.preserve_immutable_fields(&kept);
        assert_eq!(later.base.interface_source, AttributeSource::ArpReply);
    }

    #[test]
    fn preserve_immutable_fields_allows_if_name_to_be_updated_when_incoming_has_value() {
        let existing = make_interface(5, Some("Old"), None);
        let mut incoming = make_interface(5, Some("New"), None);

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(incoming.base.if_name.as_deref(), Some("New"));
    }

    #[test]
    fn preserve_immutable_fields_populates_if_name_from_legacy_null_row() {
        // Scenario: pre-existing prod row has if_name = NULL; rescan reports "eth0".
        // Incoming.if_name is Some, existing.if_name is None. Incoming value wins.
        let existing = make_interface(5, None, None);
        let mut incoming = make_interface(5, Some("eth0"), None);

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(incoming.base.if_name.as_deref(), Some("eth0"));
    }

    #[test]
    fn preserve_immutable_fields_keeps_existing_mac_when_incoming_is_none() {
        let existing = make_interface(5, Some("eth0"), Some("aa:bb:cc:dd:ee:ff"));
        let mut incoming = make_interface(5, Some("eth0"), None);

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(
            incoming.base.mac_address, existing.base.mac_address,
            "MAC address should be treated as immutable once captured from SNMP ifPhysAddress."
        );
    }

    /// A zero MAC an older daemon stored is no MAC: the next real reading replaces it even from
    /// the same source, which `REFRESHABLE = false` would otherwise refuse.
    #[test]
    fn preserve_immutable_fields_replaces_a_stored_zero_mac_with_a_real_one() {
        let existing = make_interface(5, Some("eth0"), Some("00:00:00:00:00:00"));
        let real = make_interface(5, Some("eth0"), Some("aa:bb:cc:dd:ee:ff"));
        let mut incoming = real.clone();

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(incoming.base.mac_address, real.base.mac_address);
    }

    #[test]
    fn preserve_immutable_fields_never_lets_a_zero_mac_displace_a_real_one() {
        let existing = make_interface(5, Some("eth0"), Some("aa:bb:cc:dd:ee:ff"));
        let mut incoming = make_interface(5, Some("eth0"), Some("00:00:00:00:00:00"));

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(incoming.base.mac_address, existing.base.mac_address);
    }

    #[test]
    fn preserve_immutable_fields_keeps_existing_iftable_fields_when_incoming_has_none_of_them() {
        // The shape a PROFINET DCP submission takes: matched onto an existing SNMP-walked row by
        // MAC alone, carrying none of the ifTable fields that row already has.
        let mut existing = make_interface(5, Some("eth0"), None);
        existing.base.if_type = Some(6);
        existing.base.admin_status = Some(IfAdminStatus::Up);
        existing.base.oper_status = Some(IfOperStatus::Up);
        existing.base.if_descr = Some("GigabitEthernet0/1".to_string());

        let mut incoming = Interface::new(InterfaceBase {
            host_id: existing.base.host_id,
            site_id: existing.base.site_id,
            ..Default::default()
        });

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(incoming.base.if_index, existing.base.if_index);
        assert_eq!(incoming.base.if_type, existing.base.if_type);
        assert_eq!(incoming.base.admin_status, existing.base.admin_status);
        assert_eq!(incoming.base.oper_status, existing.base.oper_status);
        assert_eq!(incoming.base.if_descr, existing.base.if_descr);
    }

    #[test]
    fn preserve_immutable_fields_allows_iftable_fields_to_be_updated_when_incoming_has_them() {
        // A real SNMP re-walk still overwrites its own prior reading — the guard only protects a
        // narrower source from clobbering fields it never claimed to know, never a real update.
        let mut existing = make_interface(5, Some("eth0"), None);
        existing.base.if_type = Some(6);
        existing.base.oper_status = Some(IfOperStatus::Up);

        let mut incoming = make_interface(5, Some("eth0"), None);
        incoming.base.if_type = Some(117);
        incoming.base.oper_status = Some(IfOperStatus::Down);

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(incoming.base.if_type, Some(117));
        assert_eq!(incoming.base.oper_status, Some(IfOperStatus::Down));
    }

    #[test]
    fn preserve_immutable_fields_copies_created_at() {
        let existing = make_interface(5, Some("eth0"), None);
        let mut incoming = make_interface(5, Some("eth0"), None);
        // Incoming has a fresh created_at; after preservation it should match the
        // existing row's created_at so the row's age is not reset on every scan.
        assert_ne!(existing.created_at, incoming.created_at);

        incoming.preserve_immutable_fields(&existing);

        assert_eq!(incoming.created_at, existing.created_at);
    }
}

/// A scan that could not finish reading a group of data must not erase what is already stored.
///
/// LLDP/CDP preservation moved to `InterfaceNeighborService::replace_candidates_from_discovery`
/// (see `interface_neighbors/service.rs`'s own tests) along with the fields it guards. What stays
/// here is FDB and VLAN membership — the two groups `preserve_uncollected_data` still covers.
#[cfg(test)]
mod preserve_uncollected_tests {
    use crate::server::interfaces::r#impl::base::{
        FdbMacs, FdbMacsValue, Interface, InterfaceBase, InterfaceDataComplete,
    };
    use crate::server::services::r#impl::patterns::ClientProbe;
    use crate::server::shared::attribution::AttributeSource;

    fn with_fdb(mac: Option<&str>) -> Interface {
        Interface::new(InterfaceBase {
            fdb_macs: mac.map(|m| {
                FdbMacs::new(
                    FdbMacsValue(vec![m.to_string()]),
                    AttributeSource::Probe(ClientProbe::Snmp),
                )
            }),
            ..Default::default()
        })
    }

    /// The reported failure, at the group that stayed on `Interface`: a truncated FDB walk
    /// produced an incoming row with no learned MACs, which overwrote a good one.
    #[test]
    fn an_incomplete_fdb_walk_keeps_the_stored_macs() {
        let existing = with_fdb(Some("00:1a:2b:00:10:00"));
        let mut incoming = with_fdb(None);

        incoming.preserve_uncollected_data(
            &existing,
            InterfaceDataComplete {
                fdb: false,
                ..Default::default()
            },
        );

        assert_eq!(incoming.base.fdb_macs, existing.base.fdb_macs);
    }

    /// The other direction, which is why this cannot simply preserve whenever the incoming value
    /// is absent: a device that genuinely lost its learned MACs reports nothing, and that has to
    /// clear — otherwise a decommissioned link is drawn for ever.
    #[test]
    fn a_complete_fdb_walk_clears_macs_that_are_gone() {
        let existing = with_fdb(Some("00:1a:2b:00:10:00"));
        let mut incoming = with_fdb(None);

        incoming.preserve_uncollected_data(&existing, InterfaceDataComplete::default());

        assert!(
            incoming.base.fdb_macs.is_none(),
            "a complete walk reporting no learned MACs is authoritative"
        );
    }

    /// An older daemon omits the flags entirely, so serde fills them in as all-complete and the
    /// upsert overwrites exactly as it did before this existed.
    #[test]
    fn an_old_daemon_payload_defaults_to_authoritative() {
        let parsed: InterfaceDataComplete = serde_json::from_str("{}").unwrap();
        assert!(parsed.all());
    }
}
