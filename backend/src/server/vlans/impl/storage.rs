use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    shared::{
        attribution::AttributeSource,
        entities::EntityDiscriminants,
        product_area::ProductArea,
        storage::{
            attributed,
            snapshot::{DiscoveryTracked, Snapshotable},
            traits::{Entity, SqlValue, Storable},
        },
        types::entities::EntitySource,
    },
    vlans::r#impl::base::{Vlan, VlanBase, VlanName, VlanNameValue},
};

impl Storable for Vlan {
    type BaseData = VlanBase;

    fn table_name() -> &'static str {
        "vlans"
    }

    /// The VLAN ID as text, so typing `20` finds VLAN 20 (and 120, 200, ...), plus its name and notes.
    fn search_predicates() -> &'static [&'static str] {
        &[
            "vlans.vlan_number::text ILIKE {}",
            "vlans.name ILIKE {}",
            "vlans.description ILIKE {}",
        ]
    }

    const HAS_SCD2: bool = true;

    fn is_live_row(&self) -> bool {
        self.valid_to.is_none()
    }

    fn new(base: Self::BaseData) -> Self {
        let now = crate::server::shared::clock::now();

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
            base:
                Self::BaseData {
                    vlan_number,
                    name,
                    description,
                    site_id,
                    organization_id,
                    source,
                    // Hydrated from the `subnet_vlans` junction on read; not a
                    // column, so nothing sent by a client is persisted.
                    subnet_ids: _,
                    // Hydrated from interfaces' native/member VLANs on read; not a column.
                    interface_ids: _,
                    tags: _, // Stored in entity_tags junction table
                },
        } = self.clone();
        let [name_value, name_source] = attributed::present_params(&name);

        Ok((
            vec![
                "id",
                "vlan_number",
                "name",
                "name_source",
                "description",
                "site_id",
                "organization_id",
                "source",
                "created_at",
                "updated_at",
                "valid_from",
                "valid_to",
                "lineage_id",
                "last_seen_at",
                "last_discovery_id",
                "first_discovery_id",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::U16(vlan_number),
                name_value,
                name_source,
                SqlValue::OptionalString(description),
                SqlValue::Uuid(site_id),
                SqlValue::Uuid(organization_id),
                SqlValue::EntitySource(source),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::Timestamp(valid_from),
                SqlValue::OptionTimestamp(valid_to),
                SqlValue::OptionalUuid(lineage_id),
                SqlValue::Timestamp(last_seen_at),
                SqlValue::OptionalUuid(last_discovery_id),
                SqlValue::OptionalUuid(first_discovery_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let vlan_number_i16: i16 = row.get("vlan_number");
        Ok(Vlan {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            last_seen_at: row.get("last_seen_at"),
            last_discovery_id: row.get("last_discovery_id"),
            first_discovery_id: row.get("first_discovery_id"),
            base: VlanBase {
                vlan_number: vlan_number_i16 as u16,
                name: attributed::read_required::<VlanNameValue>(row)?,
                description: row.get("description"),
                site_id: row.get("site_id"),
                organization_id: row.get("organization_id"),
                source: serde_json::from_value(row.get::<serde_json::Value, _>("source"))
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize source: {}", e))?,
                // Populated by `VlanService` on read, not from this row.
                subnet_ids: Vec::new(),
                interface_ids: Vec::new(),
                tags: Vec::new(), // Hydrated from entity_tags junction table
            },
        })
    }
}

impl Snapshotable for Vlan {
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
    // Vlans are top-level — no within-tracked-set FKs to remap.
}

impl DiscoveryTracked for Vlan {
    // Overrides the trait default: this type carries `EntitySource`, so a
    // manually- or system-created row must never read as stale (discovery
    // never refreshes its `last_seen_at`).
    fn is_discovery_managed(&self) -> bool {
        self.base.source.is_from_discovery()
    }

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
            &scanned.vlan_ids,
        )
    }
}

impl Entity for Vlan {
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
        EntityDiscriminants::Vlan
    }

    const ENTITY_NAME_SINGULAR: &'static str = "VLAN";
    const ENTITY_NAME_PLURAL: &'static str = "VLANs";
    const ENTITY_DESCRIPTION: &'static str = "VLANs (802.1Q virtual LANs) defined or discovered on the network. Each VLAN has a number (1-4094), a name, and an optional description, and is referenced by interfaces that participate in it.";

    fn product_area() -> ProductArea {
        ProductArea::Inventory
    }

    fn site_id(&self) -> Option<Uuid> {
        Some(self.base.site_id)
    }

    fn organization_id(&self) -> Option<Uuid> {
        Some(self.base.organization_id)
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }

    fn set_source(&mut self, source: EntitySource) {
        // A VLAN a person creates is named by that person, at the rung nothing discovery reads
        // displaces.
        if source == EntitySource::Manual {
            self.base.name = VlanName::new(self.base.name.value().clone(), AttributeSource::Manual);
        }
        self.base.source = source;
    }
}

impl Vlan {
    /// Only users reach the update handler, so a changed name is one a person typed. An
    /// unchanged one keeps the stored source: echoing back a discovered name is not renaming
    /// it, and must not lift it above the next reading.
    pub(crate) fn merge_name(&mut self, existing: &Self) {
        self.base.name = if self.base.name.value() == existing.base.name.value() {
            existing.base.name.clone()
        } else {
            VlanName::new(self.base.name.value().clone(), AttributeSource::Manual)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::types::entities::EntitySource;

    #[test]
    fn update_cannot_change_vlan_identity() {
        let existing = Vlan {
            base: VlanBase {
                vlan_number: 20,
                name: VlanName::new(
                    VlanNameValue("Servers".to_string()),
                    AttributeSource::Authored(
                        crate::server::services::r#impl::patterns::ClientProbe::Snmp,
                    ),
                ),
                site_id: Uuid::new_v4(),
                organization_id: Uuid::new_v4(),
                source: EntitySource::Manual,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut request = existing.clone();
        request.base.name = VlanName::new(
            VlanNameValue("Server farm".to_string()),
            AttributeSource::Unspecified,
        );
        request.base.description = Some("Rack A".to_string());
        request.base.tags = vec![Uuid::new_v4()];
        request.base.vlan_number = 30;
        request.base.site_id = Uuid::new_v4();
        request.base.organization_id = Uuid::new_v4();

        request.preserve_immutable_fields(&existing);

        assert_eq!(request.base.name.value().0, "Server farm");
        assert_eq!(
            request.base.name.source(),
            AttributeSource::Manual,
            "a renamed VLAN is named by the person who renamed it"
        );
        assert_eq!(request.base.description.as_deref(), Some("Rack A"));
        assert_ne!(request.base.tags, existing.base.tags, "tags stay editable");
        assert_eq!(request.base.vlan_number, 20);
        assert_eq!(request.base.site_id, existing.base.site_id);
        assert_eq!(request.base.organization_id, existing.base.organization_id);
    }

    /// The edit form sends the name back with every save. Saving a description must not turn a
    /// discovered name into the person's own, or the switch could never rename it again.
    #[test]
    fn saving_without_renaming_keeps_the_discovered_source() {
        let discovered =
            AttributeSource::Authored(crate::server::services::r#impl::patterns::ClientProbe::Snmp);
        let existing = Vlan {
            base: VlanBase {
                name: VlanName::new(VlanNameValue("Servers".to_string()), discovered),
                source: EntitySource::Discovery,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut request = existing.clone();
        request.base.name = VlanName::new(
            VlanNameValue("Servers".to_string()),
            AttributeSource::Manual,
        );
        request.base.description = Some("Rack A".to_string());

        request.preserve_immutable_fields(&existing);

        assert_eq!(request.base.name.source(), discovered);
    }
}
