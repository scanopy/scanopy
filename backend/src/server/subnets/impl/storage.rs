use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    shared::{
        attribution::{AttributeMethod, AttributeSource},
        entities::EntityDiscriminants,
        product_area::ProductArea,
        storage::{
            attributed,
            snapshot::{DiscoveryTracked, Snapshotable},
            traits::{Entity, SqlValue, Storable},
        },
        types::entities::EntitySource,
    },
    subnets::r#impl::base::{
        Subnet, SubnetBase, SubnetCidr, SubnetCidrValue, SubnetName, SubnetNameValue,
        SubnetTypeAttributed, SubnetTypeValue,
    },
};

impl Storable for Subnet {
    type BaseData = SubnetBase;

    fn table_name() -> &'static str {
        "subnets"
    }

    /// The columns the subnet list shows: a range is found by its name, its CIDR as typed, or the
    /// notes on it.
    fn search_predicates() -> &'static [&'static str] {
        &[
            "subnets.name ILIKE {}",
            "subnets.cidr ILIKE {}",
            "subnets.description ILIKE {}",
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
                    source,
                    cidr,
                    subnet_type,
                    description,
                    virtualization_service_id,
                    // Hydrated from the `subnet_vlans` junction on read; not a column.
                    vlan_ids: _,
                    tags: _, // Stored in entity_tags junction table
                },
        } = self.clone();

        let [cidr_value, cidr_source] = attributed::present_params(&cidr);
        let [name_value, name_source] = attributed::present_params(&name);
        let [subnet_type_value, subnet_type_source] = attributed::present_params(&subnet_type);

        Ok((
            vec![
                "id",
                "name",
                "name_source",
                "description",
                "cidr",
                "cidr_source",
                "source",
                "subnet_type",
                "subnet_type_source",
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
                name_value,
                name_source,
                SqlValue::OptionalString(description),
                cidr_value,
                cidr_source,
                SqlValue::EntitySource(source),
                subnet_type_value,
                subnet_type_source,
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
        let subnet_type = attributed::read_required::<SubnetTypeValue>(row)?;
        let name = attributed::read_required::<SubnetNameValue>(row)?;
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
                name,
                description: row.get("description"),
                site_id: row.get("site_id"),
                source,
                cidr,
                subnet_type,
                virtualization_service_id: row.get("virtualization_service_id"),
                vlan_ids: Vec::new(), // Hydrated from the `subnet_vlans` junction
                tags: Vec::new(),     // Hydrated from entity_tags junction table
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

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Subnet
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Subnet";
    const ENTITY_NAME_PLURAL: &'static str = "Subnets";
    const ENTITY_DESCRIPTION: &'static str =
        "IP subnets within sites. Define address ranges and organize hosts by subnet.";

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
            // The name and type a person gave the subnet they created are theirs too.
            self.base.name =
                SubnetName::new(self.base.name.value().clone(), AttributeSource::Manual);
            self.base.subnet_type =
                SubnetTypeAttributed::new(*self.base.subnet_type.value(), AttributeSource::Manual);
        }
        self.base.source = source;
    }
}

impl Subnet {
    /// The rung the caller supplied, never lower than the one already stored. This used to
    /// read a *changed* CIDR as proof a person had typed it, on the premise that discovery never
    /// moves an existing subnet's range. That premise no longer holds: a real netmask now
    /// corrects a range Scanopy only inferred. Deciding whose edit this was belongs to the layer
    /// that knows — `update_subnet` stamps the manual rung, and daemons cannot call it.
    ///
    /// Through the applier rather than a `max()` on the source alone: the pair is one field now,
    /// and raising the stored rung over the incoming *value* is how a guess used to end up
    /// labelled as a reading.
    pub(crate) fn merge_cidr(&mut self, existing: &Self) {
        let mut merged = existing.base.cidr.clone();
        merged.apply_in_place(self.base.cidr.clone());
        self.base.cidr = merged;
    }

    /// Source is set at creation and kept, except on an inferred subnet whose range has just
    /// been settled: read by a daemon, or confirmed or corrected by a person (`update_subnet`
    /// sends `Manual`). The range is the thing that was assumed, so settling it confirms the
    /// subnet, the way a scan reaching an inferred host confirms the host. Reads the merged
    /// range, so it follows [`Self::merge_cidr`] in the registry.
    pub(crate) fn merge_source(&mut self, existing: &Self) {
        let incoming_source =
            std::mem::replace(&mut self.base.source, existing.base.source.clone());
        if matches!(existing.base.source, EntitySource::Inferred { .. })
            && self.base.cidr.source().method() != AttributeMethod::Inferred
        {
            self.base.source = if incoming_source.confirmation().is_unconfirmed() {
                EntitySource::Discovery
            } else {
                incoming_source
            };
        }
    }

    /// The same as [`Self::merge_cidr`]: `update_subnet` stamps `Manual` on a value a person
    /// changed, and the applier keeps everything else at the stored rung.
    pub(crate) fn merge_name(&mut self, existing: &Self) {
        let mut name = existing.base.name.clone();
        name.apply_in_place(self.base.name.clone());
        self.base.name = name;
    }

    /// See [`Self::merge_name`].
    pub(crate) fn merge_subnet_type(&mut self, existing: &Self) {
        let mut subnet_type = existing.base.subnet_type.clone();
        subnet_type.apply_in_place(self.base.subnet_type.clone());
        self.base.subnet_type = subnet_type;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::services::r#impl::patterns::ClientProbe;
    use crate::server::subnets::r#impl::types::SubnetType;

    /// The edit form sends the name and type back with every save, as plain values. Saving a
    /// description must not drop the source a scan recorded, or the controller could no longer
    /// update a name it set.
    #[test]
    fn saving_without_renaming_keeps_the_discovered_sources() {
        let controller = AttributeSource::Authored(ClientProbe::UnifiController);
        let existing = Subnet::new(SubnetBase {
            name: SubnetName::new(SubnetNameValue("Servers".to_string()), controller),
            subnet_type: SubnetTypeAttributed::new(SubnetTypeValue(SubnetType::Lan), controller),
            ..Default::default()
        });

        let mut request = existing.clone();
        request.base.name = SubnetName::unattributed("Servers");
        request.base.subnet_type = SubnetTypeAttributed::unattributed(SubnetType::Lan);
        request.base.description = Some("Rack A".to_string());

        request.preserve_immutable_fields(&existing);

        assert_eq!(request.base.name.source(), controller);
        assert_eq!(request.base.subnet_type.source(), controller);
    }

    fn assumed_range() -> Subnet {
        Subnet::new(SubnetBase {
            cidr: SubnetCidr::new(
                SubnetCidrValue("10.0.50.0/24".parse().unwrap()),
                AttributeSource::LldpNeighbourAddress,
            ),
            source: EntitySource::Inferred {
                via: crate::server::shared::types::entities::InferredVia::ReportedAddress,
            },
            ..Default::default()
        })
    }

    #[test]
    fn a_daemon_reading_an_assumed_range_confirms_the_subnet() {
        let existing = assumed_range();
        let mut incoming = existing.clone();
        incoming.base.cidr = SubnetCidr::new(
            SubnetCidrValue("10.0.50.0/24".parse().unwrap()),
            AttributeSource::DaemonSelfReport,
        );
        incoming.base.source = EntitySource::Discovery;
        incoming.preserve_immutable_fields(&existing);
        assert_eq!(incoming.base.source, EntitySource::Discovery);
    }

    #[test]
    fn a_person_confirming_an_assumed_range_makes_the_subnet_manual() {
        let existing = assumed_range();
        let mut incoming = existing.clone();
        incoming.base.cidr =
            SubnetCidr::new(existing.base.cidr.value().clone(), AttributeSource::Manual);
        incoming.base.source = EntitySource::Manual;
        incoming.preserve_immutable_fields(&existing);
        assert_eq!(incoming.base.source, EntitySource::Manual);
    }

    #[test]
    fn another_assumption_leaves_the_subnet_unconfirmed() {
        let existing = assumed_range();
        let mut incoming = existing.clone();
        incoming.base.source = EntitySource::Discovery;
        incoming.preserve_immutable_fields(&existing);
        assert!(incoming.base.source.confirmation().is_unconfirmed());
    }
}
