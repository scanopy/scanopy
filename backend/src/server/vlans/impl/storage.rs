use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            snapshot::{DiscoveryTracked, Snapshotable},
            traits::{Entity, SqlValue, Storable},
        },
    },
    vlans::r#impl::base::{Vlan, VlanBase},
};

/// CSV row representation for Vlan export
#[derive(Serialize)]
pub struct VlanCsvRow {
    pub id: Uuid,
    pub vlan_number: u16,
    pub name: String,
    pub description: Option<String>,
    pub site_id: Uuid,
    pub organization_id: Uuid,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Vlan {
    type BaseData = VlanBase;

    fn table_name() -> &'static str {
        "vlans"
    }

    const HAS_SCD2: bool = true;

    fn is_live_row(&self) -> bool {
        self.valid_to.is_none()
    }

    fn new(base: Self::BaseData) -> Self {
        let now = chrono::Utc::now();

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
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "vlan_number",
                "name",
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
                SqlValue::String(name),
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
                name: row.get("name"),
                description: row.get("description"),
                site_id: row.get("site_id"),
                organization_id: row.get("organization_id"),
                source: serde_json::from_value(row.get::<serde_json::Value, _>("source"))
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize source: {}", e))?,
                // Populated by `VlanService` from the junction, not from this row.
                subnet_ids: Vec::new(),
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

    type CsvRow = VlanCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        VlanCsvRow {
            id: self.id,
            vlan_number: self.base.vlan_number,
            name: self.base.name.clone(),
            description: self.base.description.clone(),
            site_id: self.base.site_id,
            organization_id: self.base.organization_id,
            source: serde_json::to_string(&self.base.source).unwrap_or_default(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Vlan
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Vlan";
    const ENTITY_NAME_PLURAL: &'static str = "Vlans";
    const ENTITY_DESCRIPTION: &'static str = "VLANs (802.1Q virtual LANs) defined or discovered on the network. Each VLAN has a number (1-4094), a name, and an optional description, and is referenced by interfaces that participate in it.";

    fn entity_category() -> EntityCategory {
        EntityCategory::NetworkInfrastructure
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
}
