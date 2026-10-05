use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use std::str::FromStr;
use uuid::Uuid;

use crate::server::{
    shared::{
        attribution::AttributeSource,
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            attributed,
            snapshot::{DiscoveryTracked, Snapshotable},
            traits::{Entity, SqlValue, Storable},
        },
        types::{entities::EntitySource, metadata::HasId},
    },
    subnets::r#impl::{
        base::{Subnet, SubnetBase, SubnetCidr, SubnetCidrValue},
        types::SubnetType,
    },
};

/// CSV row representation for Subnet export
#[derive(Serialize)]
pub struct SubnetCsvRow {
    pub id: Uuid,
    pub name: String,
    pub cidr: String,
    pub subnet_type: String,
    pub cidr_source: String,
    pub description: Option<String>,
    pub site_id: Uuid,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Subnet {
    type BaseData = SubnetBase;

    fn table_name() -> &'static str {
        "subnets"
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
                    name,
                    site_id,
                    source,
                    cidr,
                    subnet_type,
                    description,
                    virtualization_service_id,
                    tags: _, // Stored in entity_tags junction table
                },
        } = self.clone();

        let [cidr_value, cidr_source] = attributed::present_params(&cidr);

        Ok((
            vec![
                "id",
                "name",
                "description",
                "cidr",
                "cidr_source",
                "source",
                "subnet_type",
                "virtualization_service_id",
                "site_id",
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
                SqlValue::String(name),
                SqlValue::OptionalString(description),
                cidr_value,
                cidr_source,
                SqlValue::EntitySource(source),
                SqlValue::String(subnet_type.id().to_string()),
                SqlValue::OptionalUuid(virtualization_service_id),
                SqlValue::Uuid(site_id),
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
        // Parse fields safely
        let subnet_type = SubnetType::from_str(&row.get::<String, _>("subnet_type"))
            .map_err(|e| anyhow::anyhow!("Failed to parse subnet_type: {}", e))?;
        // A CIDR is required, so unlike an optional attribute an absent one is a real error.
        // The source half degrades to `Unspecified` inside `AttributeSource`'s own `Deserialize`,
        // which is what keeps a rung a newer binary wrote from failing the whole row.
        let cidr = attributed::read_required::<SubnetCidrValue>(row)?;
        let source: EntitySource =
            serde_json::from_value(row.get::<serde_json::Value, _>("source"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize source: {}", e))?;

        Ok(Subnet {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            last_seen_at: row.get("last_seen_at"),
            last_discovery_id: row.get("last_discovery_id"),
            first_discovery_id: row.get("first_discovery_id"),
            base: SubnetBase {
                name: row.get("name"),
                description: row.get("description"),
                site_id: row.get("site_id"),
                source,
                cidr,
                subnet_type,
                virtualization_service_id: row.get("virtualization_service_id"),
                tags: Vec::new(), // Hydrated from entity_tags junction table
            },
        })
    }
}

impl Snapshotable for Subnet {
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
    // Subnets are top-level — no within-tracked-set FKs to remap.
}

impl DiscoveryTracked for Subnet {
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
        // Swept and found alike: the run touched both, whatever the digest counts as coverage.
        let ids: Vec<Uuid> = scanned
            .subnet_ids
            .iter()
            .chain(&scanned.found_subnet_ids)
            .copied()
            .collect();
        crate::server::shared::storage::filter::StorableFilter::<Self>::new_from_uuids_column(
            "id", &ids,
        )
    }
}

impl Entity for Subnet {
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

    type CsvRow = SubnetCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        SubnetCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            cidr: self.base.cidr.value().to_string(),
            subnet_type: self.base.subnet_type.id().to_string(),
            cidr_source: self.base.cidr.source().to_string(),
            description: self.base.description.clone(),
            site_id: self.base.site_id,
            source: format!("{:?}", self.base.source),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Subnet
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Subnet";
    const ENTITY_NAME_PLURAL: &'static str = "Subnets";
    const ENTITY_DESCRIPTION: &'static str =
        "IP subnets within sites. Define address ranges and organize hosts by subnet.";

    fn entity_category() -> EntityCategory {
        EntityCategory::NetworkInfrastructure
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

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }

    fn set_source(&mut self, source: EntitySource) {
        // A range a person typed into Scanopy is an assertion, not a reading, and it sits at the
        // top of the ladder — the same act that makes the row `Manual` is the one that makes its
        // CIDR `Manual`, so the two cannot be set apart from each other.
        if source == EntitySource::Manual {
            let asserted = SubnetCidr::new(self.base.cidr.value().clone(), AttributeSource::Manual);
            self.base.cidr.apply_in_place(asserted);
        }
        self.base.source = source;
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        // source is set at creation time (Manual or Discovery), cannot be changed
        self.base.source = existing.base.source.clone();
        self.created_at = existing.created_at;
        self.updated_at = existing.updated_at;

        // The rung the caller supplied, never lower than the one already stored. This used to
        // read a *changed* CIDR as proof a person had typed it, on the premise that discovery never
        // moves an existing subnet's range. That premise no longer holds: a real netmask now
        // corrects a range Scanopy only inferred. Deciding whose edit this was belongs to the layer
        // that knows — `update_subnet` stamps the manual rung, and daemons cannot call it.
        //
        // Through the applier rather than a `max()` on the source alone: the pair is one field now,
        // and raising the stored rung over the incoming *value* is how a guess used to end up
        // labelled as a reading.
        let mut merged = existing.base.cidr.clone();
        merged.apply_in_place(self.base.cidr.clone());
        self.base.cidr = merged;
    }
}
