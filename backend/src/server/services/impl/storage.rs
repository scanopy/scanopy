use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    services::r#impl::{
        base::{Service, ServiceBase},
        definitions::ServiceDefinition,
        virtualization::ServiceVirtualization,
    },
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            child::ChildStorableEntity,
            snapshot::{DiscoveryTracked, FkMaps, Snapshotable},
            traits::{Entity, SqlValue, Storable},
        },
        types::entities::EntitySource,
    },
};

/// CSV row representation for Service export (excludes nested bindings)
#[derive(Serialize)]
pub struct ServiceCsvRow {
    pub id: Uuid,
    pub name: String,
    pub service_definition: String,
    pub host_id: Uuid,
    pub site_id: Uuid,
    pub source: String,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Service {
    type BaseData = ServiceBase;

    fn table_name() -> &'static str {
        "services"
    }

    /// A service is most often looked up by its own name or by what it is
    /// (`service_definition`, e.g. "Postgres"), but "everything on that box"
    /// is just as common a reflex — hence the host-name fragment. `EXISTS`
    /// keeps the row count intact for the paginated `COUNT(*)`. Services carry
    /// no description column, so there is nothing else on the row to match.
    fn search_predicates() -> &'static [&'static str] {
        &[
            "services.name ILIKE {}",
            "services.service_definition ILIKE {}",
            "EXISTS (SELECT 1 FROM hosts h WHERE h.id = services.host_id \
             AND h.valid_to IS NULL AND h.name ILIKE {})",
        ]
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
                    host_id,
                    service_definition,
                    virtualization_metadata,
                    virtualization_service_id,
                    bindings: _, // Bindings stored in separate table, managed by BindingStorage
                    source,
                    tags: _, // Stored in entity_tags junction table
                    position,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "name",
                "site_id",
                "host_id",
                "service_definition",
                "virtualization_metadata",
                "virtualization_service_id",
                "source",
                "position",
                "valid_from",
                "valid_to",
                "lineage_id",
                "last_seen_at",
                "last_discovery_id",
                "first_discovery_id",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(name),
                SqlValue::Uuid(site_id),
                SqlValue::Uuid(host_id),
                SqlValue::ServiceDefinition(service_definition),
                SqlValue::OptionalServiceVirtualization(virtualization_metadata),
                SqlValue::OptionalUuid(virtualization_service_id),
                SqlValue::EntitySource(source),
                SqlValue::I32(position),
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
        let service_definition: Box<dyn ServiceDefinition> =
            serde_json::from_str(&row.get::<String, _>("service_definition"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize service_definition: {}", e))?;
        // Decoded as Option, unlike the previous non-Option get, which only ever worked because
        // every write path stored a JSONB 'null' — a real SQL NULL in this column would have
        // failed the decode outright.
        let virtualization_metadata: Option<ServiceVirtualization> =
            match row.get::<Option<serde_json::Value>, _>("virtualization_metadata") {
                Some(v) => serde_json::from_value(v).map_err(|e| {
                    anyhow::anyhow!("Failed to deserialize virtualization_metadata: {}", e)
                })?,
                None => None,
            };
        let source: EntitySource =
            serde_json::from_value(row.get::<serde_json::Value, _>("source"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize source: {}", e))?;

        Ok(Service {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            last_seen_at: row.get("last_seen_at"),
            last_discovery_id: row.get("last_discovery_id"),
            first_discovery_id: row.get("first_discovery_id"),
            base: ServiceBase {
                name: row.get("name"),
                site_id: row.get("site_id"),
                host_id: row.get("host_id"),
                service_definition,
                virtualization_metadata,
                virtualization_service_id: row.get("virtualization_service_id"),
                bindings: Vec::new(), // Bindings loaded separately by ServiceService via BindingStorage
                tags: Vec::new(),     // Hydrated from entity_tags junction table
                source,
                position: row.get("position"),
            },
        })
    }
}

impl Snapshotable for Service {
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

    fn remap_fks_for_clone(&mut self, maps: &FkMaps) {
        if let Some(closed) = maps.hosts.get(&self.base.host_id) {
            self.base.host_id = *closed;
        }
    }
}

impl DiscoveryTracked for Service {
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
            &scanned.service_ids,
        )
    }
}

impl Entity for Service {
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

    type CsvRow = ServiceCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        ServiceCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            service_definition: self.base.service_definition.id().to_string(),
            host_id: self.base.host_id,
            site_id: self.base.site_id,
            source: format!("{:?}", self.base.source),
            position: self.base.position,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Service
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Service";
    const ENTITY_NAME_PLURAL: &'static str = "Services";
    const ENTITY_DESCRIPTION: &'static str = "Services running on hosts. Detected or manually added services like databases, web servers, etc.";

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
        self.base.source = source;
    }

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        // source is set at creation time (Manual or Discovery), cannot be changed
        self.base.source = existing.base.source.clone();
        // Preserve virtualization if not explicitly set (discovery-managed fields). Both halves
        // are preserved independently: a caller that supplies neither must not lose the
        // container's identity or its owning runtime.
        if self.base.virtualization_metadata.is_none() {
            self.base.virtualization_metadata = existing.base.virtualization_metadata.clone();
        }
        if self.base.virtualization_service_id.is_none() {
            self.base.virtualization_service_id = existing.base.virtualization_service_id;
        }
    }
}

impl ChildStorableEntity for Service {
    fn parent_column() -> &'static str {
        "host_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.host_id
    }
}
