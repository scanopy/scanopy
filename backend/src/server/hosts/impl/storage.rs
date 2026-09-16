use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    hosts::r#impl::{
        base::{Host, HostBase},
        name::host_name_from_parts,
        virtualization::HostVirtualization,
    },
    shared::{
        attribution,
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            attributed,
            snapshot::{DiscoveryTracked, Snapshotable},
            traits::{Entity, SqlValue, Storable},
        },
        types::entities::EntitySource,
    },
};

/// CSV row representation for Host export
#[derive(Serialize)]
pub struct HostCsvRow {
    pub id: Uuid,
    pub name: String,
    pub hostname: Option<String>,
    pub description: Option<String>,
    pub network_id: Uuid,
    pub source: String,
    pub hidden: bool,
    // Everything the device reported about itself. Field order is column order — headers are
    // derived from these names — so the two timestamps stay last, as they are on every other
    // CsvRow.
    pub sys_descr: Option<String>,
    pub sys_object_id: Option<String>,
    pub sys_location: Option<String>,
    pub sys_contact: Option<String>,
    pub management_url: Option<String>,
    pub chassis_id: Option<String>,
    pub sys_name: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub firmware_revision: Option<String>,
    pub software_revision: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Host {
    type BaseData = HostBase;

    fn table_name() -> &'static str {
        "hosts"
    }

    /// Spans what an operator actually types when hunting for a host: its
    /// name/hostname, a snippet of its description, an IP, a MAC, or the name
    /// of something running on it.
    ///
    /// The MAC predicates are not a convenience. A device identified only by
    /// its MAC — one with no address at all — was unfindable by the single
    /// identity it has, so the search box could not reach a host the fleet
    /// holds. Both tables carry one, and which table depends on how the device
    /// was found, so searching one would leave half of them unreachable.
    /// Matched as text, so a partial address or a bare OUI prefix works the way
    /// a partial IP already does.
    ///
    /// Every rung of the [`Host::display_name`] ladder is in here, which is why
    /// `sys_name` and `chassis_id` are covered even though no column shows them
    /// by default: a host that never got a name is *listed* under one of them,
    /// and searching for the title on screen has to find the host wearing it.
    ///
    /// Children are matched with `EXISTS` rather than a JOIN so a host with
    /// many IPs or services is not duplicated in the result set — which would
    /// also corrupt the paginated `COUNT(*)`. The `valid_to IS NULL` guards
    /// keep closed SCD2 copies from matching, so a host stops being findable
    /// by an IP it no longer holds.
    ///
    /// [`Host::display_name`]: crate::server::hosts::r#impl::base::Host::display_name
    fn search_predicates() -> &'static [&'static str] {
        &[
            "hosts.name ILIKE {}",
            "hosts.hostname ILIKE {}",
            "hosts.sys_name ILIKE {}",
            "hosts.chassis_id ILIKE {}",
            "hosts.description ILIKE {}",
            "EXISTS (SELECT 1 FROM ip_addresses ia WHERE ia.host_id = hosts.id \
             AND ia.valid_to IS NULL AND host(ia.ip_address) ILIKE {})",
            "EXISTS (SELECT 1 FROM ip_addresses ia WHERE ia.host_id = hosts.id \
             AND ia.valid_to IS NULL AND ia.mac_address::text ILIKE {})",
            "EXISTS (SELECT 1 FROM interfaces i WHERE i.host_id = hosts.id \
             AND i.valid_to IS NULL AND i.mac_address::text ILIKE {})",
            "EXISTS (SELECT 1 FROM services s WHERE s.host_id = hosts.id \
             AND s.valid_to IS NULL AND s.name ILIKE {})",
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
        // Exhaustive destructuring ensures compile error if HostBase changes
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
                    description,
                    hostname,
                    network_id,
                    hidden,
                    source,
                    virtualization_metadata,
                    virtualization_service_id,
                    tags: _, // Stored in entity_tags junction table
                    sys_descr,
                    sys_object_id,
                    sys_location,
                    sys_contact,
                    management_url,
                    chassis_id,
                    sys_name,
                    manufacturer,
                    model,
                    serial_number,
                    firmware_revision,
                    software_revision,
                    credential_assignments: _, // Stored in host_credentials junction table
                },
        } = self.clone();

        // Each provenanced pair hands out its two columns and its two values together, so the two
        // vectors below cannot drift apart on one of them.
        let [name_value, name_source] = attributed::present_params(&name);
        let [hostname_value, hostname_source] = attributed::optional_params(&hostname);
        let [sys_descr_value, sys_descr_source] = attributed::optional_params(&sys_descr);
        let [sys_object_id_value, sys_object_id_source] =
            attributed::optional_params(&sys_object_id);
        let [sys_location_value, sys_location_source] = attributed::optional_params(&sys_location);
        let [sys_contact_value, sys_contact_source] = attributed::optional_params(&sys_contact);
        let [management_url_value, management_url_source] =
            attributed::optional_params(&management_url);
        let [chassis_id_value, chassis_id_source] = attributed::optional_params(&chassis_id);
        let [sys_name_value, sys_name_source] = attributed::optional_params(&sys_name);
        let [manufacturer_value, manufacturer_source] = attributed::optional_params(&manufacturer);
        let [model_value, model_source] = attributed::optional_params(&model);
        let [serial_number_value, serial_number_source] =
            attributed::optional_params(&serial_number);
        let [firmware_revision_value, firmware_revision_source] =
            attributed::optional_params(&firmware_revision);
        let [software_revision_value, software_revision_source] =
            attributed::optional_params(&software_revision);

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "name",
                "name_source",
                "description",
                "network_id",
                "source",
                "hostname",
                "hostname_source",
                "hidden",
                "virtualization_metadata",
                "virtualization_service_id",
                "sys_descr",
                "sys_descr_source",
                "sys_object_id",
                "sys_object_id_source",
                "sys_location",
                "sys_location_source",
                "sys_contact",
                "sys_contact_source",
                "management_url",
                "management_url_source",
                "chassis_id",
                "chassis_id_source",
                "sys_name",
                "sys_name_source",
                "manufacturer",
                "manufacturer_source",
                "model",
                "model_source",
                "serial_number",
                "serial_number_source",
                "firmware_revision",
                "firmware_revision_source",
                "software_revision",
                "software_revision_source",
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
                name_value,
                name_source,
                SqlValue::OptionalString(description),
                SqlValue::Uuid(network_id),
                SqlValue::EntitySource(source),
                hostname_value,
                hostname_source,
                SqlValue::Bool(hidden),
                SqlValue::OptionalHostVirtualization(virtualization_metadata),
                SqlValue::OptionalUuid(virtualization_service_id),
                sys_descr_value,
                sys_descr_source,
                sys_object_id_value,
                sys_object_id_source,
                sys_location_value,
                sys_location_source,
                sys_contact_value,
                sys_contact_source,
                management_url_value,
                management_url_source,
                chassis_id_value,
                chassis_id_source,
                sys_name_value,
                sys_name_source,
                manufacturer_value,
                manufacturer_source,
                model_value,
                model_source,
                serial_number_value,
                serial_number_source,
                firmware_revision_value,
                firmware_revision_source,
                software_revision_value,
                software_revision_source,
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
        // Parse JSON fields safely
        let source: EntitySource =
            serde_json::from_value(row.get::<serde_json::Value, _>("source"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize source: {}", e))?;
        // virtualization_metadata is a nullable JSONB column, so decode it as Option: a SQL NULL
        // (as opposed to a JSONB 'null') must map to None rather than panic on a non-Option get.
        let virtualization_metadata: Option<HostVirtualization> =
            match row.get::<Option<serde_json::Value>, _>("virtualization_metadata") {
                Some(v) => serde_json::from_value(v).map_err(|e| {
                    anyhow::anyhow!("Failed to deserialize virtualization_metadata: {}", e)
                })?,
                None => None,
            };

        // `host_name_from_parts` rather than a plain construction: a blank name collapses to
        // unnamed whatever rung the column claims, and an address rung whose value is not an
        // address degrades instead of asserting something false.
        let name = host_name_from_parts(
            row.get::<String, _>("name"),
            attributed::read_source(row, "name_source")?,
        );

        Ok(Host {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            last_seen_at: row.get("last_seen_at"),
            last_discovery_id: row.get("last_discovery_id"),
            first_discovery_id: row.get("first_discovery_id"),
            base: HostBase {
                name,
                description: row.get("description"),
                network_id: row.get("network_id"),
                source,
                hostname: attributed::read_optional(row)?,
                hidden: row.get("hidden"),
                virtualization_metadata,
                virtualization_service_id: row.get("virtualization_service_id"),
                tags: Vec::new(), // Hydrated from entity_tags junction table
                sys_descr: attributed::read_optional(row)?,
                sys_object_id: attributed::read_optional(row)?,
                sys_location: attributed::read_optional(row)?,
                sys_contact: attributed::read_optional(row)?,
                management_url: attributed::read_optional(row)?,
                chassis_id: attributed::read_optional(row)?,
                sys_name: attributed::read_optional(row)?,
                manufacturer: attributed::read_optional(row)?,
                model: attributed::read_optional(row)?,
                serial_number: attributed::read_optional(row)?,
                firmware_revision: attributed::read_optional(row)?,
                software_revision: attributed::read_optional(row)?,
                credential_assignments: Vec::new(), // Hydrated from host_credentials junction table
            },
        })
    }
}

impl Entity for Host {
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

    type CsvRow = HostCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        HostCsvRow {
            id: self.id,
            name: self.base.name.to_string(),
            hostname: attribution::text_of(&self.base.hostname),
            description: self.base.description.clone(),
            network_id: self.base.network_id,
            source: format!("{:?}", self.base.source),
            hidden: self.base.hidden,
            sys_descr: attribution::text_of(&self.base.sys_descr),
            sys_object_id: attribution::text_of(&self.base.sys_object_id),
            sys_location: attribution::text_of(&self.base.sys_location),
            sys_contact: attribution::text_of(&self.base.sys_contact),
            management_url: attribution::text_of(&self.base.management_url),
            chassis_id: attribution::text_of(&self.base.chassis_id),
            sys_name: attribution::text_of(&self.base.sys_name),
            manufacturer: attribution::text_of(&self.base.manufacturer),
            model: attribution::text_of(&self.base.model),
            serial_number: attribution::text_of(&self.base.serial_number),
            firmware_revision: attribution::text_of(&self.base.firmware_revision),
            software_revision: attribution::text_of(&self.base.software_revision),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Host
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Host";
    const ENTITY_NAME_PLURAL: &'static str = "Hosts";
    const ENTITY_DESCRIPTION: &'static str =
        "Network hosts (devices). Manage discovered or manually created hosts on your network.";

    fn entity_category() -> EntityCategory {
        EntityCategory::NetworkInfrastructure
    }

    fn network_id(&self) -> Option<Uuid> {
        Some(self.base.network_id)
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
        // source is set at creation time (Manual or Discovery), cannot be changed — with one
        // exception. `Inferred` says "nothing has ever contacted this device", and the moment a
        // scan does, that stops being true. Pinning it would leave a host that answers SNMP still
        // badged as second-hand for ever, which is the opposite of what the rung is for.
        self.base.source = match (&existing.base.source, &self.base.source) {
            (EntitySource::Inferred, incoming) if incoming.is_from_discovery() => incoming.clone(),
            (existing_source, _) => existing_source.clone(),
        };
        self.created_at = existing.created_at;
        self.updated_at = existing.updated_at;
    }
}

impl Snapshotable for Host {
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
    // Hosts are top-level — no within-tracked-set FKs to remap.
}

impl DiscoveryTracked for Host {
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
            &scanned.host_ids,
        )
    }
}
