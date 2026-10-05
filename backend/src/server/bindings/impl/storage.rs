use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    bindings::r#impl::base::{Binding, BindingBase, BindingType},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            snapshot::{DiscoveryTracked, FkMaps, Snapshotable},
            traits::{Entity, SqlValue, Storable},
        },
    },
};

/// CSV row representation for Binding export
#[derive(Serialize)]
pub struct BindingCsvRow {
    pub id: Uuid,
    pub service_id: Uuid,
    pub binding_type: String,
    pub ip_address_id: Option<Uuid>,
    pub port_id: Option<Uuid>,
    pub site_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Binding {
    type BaseData = BindingBase;

    fn table_name() -> &'static str {
        "bindings"
    }

    const HAS_SCD2: bool = true;

    fn is_live_row(&self) -> bool {
        self.valid_to.is_none()
    }

    fn new(base: Self::BaseData) -> Self {
        Binding::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let (binding_type, ip_address_id, port_id) = match self.base.binding_type {
            BindingType::IPAddress { ip_address_id } => ("IPAddress", Some(ip_address_id), None),
            BindingType::Port {
                port_id,
                ip_address_id,
            } => ("Port", ip_address_id, Some(port_id)),
        };

        Ok((
            vec![
                "id",
                "service_id",
                "site_id",
                "binding_type",
                "ip_address_id",
                "port_id",
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
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.service_id),
                SqlValue::Uuid(self.base.site_id),
                SqlValue::String(binding_type.to_string()),
                SqlValue::OptionalUuid(ip_address_id),
                SqlValue::OptionalUuid(port_id),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
                SqlValue::Timestamp(self.valid_from),
                SqlValue::OptionTimestamp(self.valid_to),
                SqlValue::OptionalUuid(self.lineage_id),
                SqlValue::Timestamp(self.last_seen_at),
                SqlValue::OptionalUuid(self.last_discovery_id),
                SqlValue::OptionalUuid(self.first_discovery_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let id: Uuid = row.get("id");
        let service_id: Uuid = row.get("service_id");
        let site_id: Uuid = row.get("site_id");
        let created_at: DateTime<Utc> = row.get("created_at");
        let updated_at: DateTime<Utc> = row.get("updated_at");
        let binding_type_str: String = row.get("binding_type");
        let ip_address_id: Option<Uuid> = row.get("ip_address_id");
        let port_id: Option<Uuid> = row.get("port_id");

        let binding_type = match binding_type_str.as_str() {
            "IPAddress" => {
                let ip_address_id = ip_address_id
                    .ok_or_else(|| anyhow::anyhow!("IPAddress binding missing ip_address_id"))?;
                BindingType::IPAddress { ip_address_id }
            }
            "Port" => {
                let port_id =
                    port_id.ok_or_else(|| anyhow::anyhow!("Port binding missing port_id"))?;
                BindingType::Port {
                    port_id,
                    ip_address_id,
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unknown binding type: {}",
                    binding_type_str
                ));
            }
        };

        Ok(Binding {
            id,
            created_at,
            updated_at,
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            last_seen_at: row.get("last_seen_at"),
            last_discovery_id: row.get("last_discovery_id"),
            first_discovery_id: row.get("first_discovery_id"),
            base: BindingBase {
                service_id,
                site_id,
                binding_type,
            },
        })
    }
}

impl Snapshotable for Binding {
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
        if let Some(closed) = maps.services.get(&self.base.service_id) {
            self.base.service_id = *closed;
        }
        // Remap inside the binding_type variant.
        match &mut self.base.binding_type {
            BindingType::IPAddress { ip_address_id } => {
                if let Some(closed) = maps.ip_addresses.get(ip_address_id) {
                    *ip_address_id = *closed;
                }
            }
            BindingType::Port {
                port_id,
                ip_address_id,
            } => {
                if let Some(closed) = maps.ports.get(port_id) {
                    *port_id = *closed;
                }
                if let Some(ip) = ip_address_id
                    && let Some(closed) = maps.ip_addresses.get(ip)
                {
                    *ip_address_id = Some(*closed);
                }
            }
        }
    }
}

impl DiscoveryTracked for Binding {
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
            &scanned.binding_ids,
        )
    }
}

impl Entity for Binding {
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

    type CsvRow = BindingCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        let (binding_type, ip_address_id, port_id) = match self.base.binding_type {
            BindingType::IPAddress { ip_address_id } => ("IPAddress", Some(ip_address_id), None),
            BindingType::Port {
                port_id,
                ip_address_id,
            } => ("Port", ip_address_id, Some(port_id)),
        };
        BindingCsvRow {
            id: self.id,
            service_id: self.base.service_id,
            binding_type: binding_type.to_string(),
            ip_address_id,
            port_id,
            site_id: self.base.site_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Binding
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Binding";
    const ENTITY_NAME_PLURAL: &'static str = "Bindings";
    const ENTITY_DESCRIPTION: &'static str = "Service bindings linking services to IP addresses and/or ports. Defines where a service is accessible.";

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
}

#[cfg(test)]
mod remap_tests {
    use super::*;
    use crate::server::bindings::r#impl::base::{Binding, BindingBase, BindingType};
    use crate::server::shared::storage::snapshot::{FkMaps, Snapshotable};

    // Snapshot close-and-clone must rewrite a cloned binding's FK columns to the
    // closed copies of its service / ip_address / port, so the historical binding
    // references the historical parents (not the live ones). This is the
    // `service_bindings -> ip_addresses` cascade the task called out.
    #[test]
    fn ip_address_binding_remaps_service_and_ip_address() {
        let (live_service, closed_service) = (Uuid::new_v4(), Uuid::new_v4());
        let (live_ip, closed_ip) = (Uuid::new_v4(), Uuid::new_v4());

        let mut binding = Binding::new(BindingBase::new(
            live_service,
            Uuid::new_v4(),
            BindingType::IPAddress {
                ip_address_id: live_ip,
            },
        ));

        let mut maps = FkMaps::default();
        maps.services.insert(live_service, closed_service);
        maps.ip_addresses.insert(live_ip, closed_ip);

        binding.remap_fks_for_clone(&maps);

        assert_eq!(binding.base.service_id, closed_service);
        match binding.base.binding_type {
            BindingType::IPAddress { ip_address_id } => assert_eq!(ip_address_id, closed_ip),
            other => panic!("unexpected binding type: {other:?}"),
        }
    }

    #[test]
    fn port_binding_remaps_port_and_optional_ip_address() {
        let (live_port, closed_port) = (Uuid::new_v4(), Uuid::new_v4());
        let (live_ip, closed_ip) = (Uuid::new_v4(), Uuid::new_v4());

        let mut binding = Binding::new(BindingBase::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            BindingType::Port {
                port_id: live_port,
                ip_address_id: Some(live_ip),
            },
        ));

        let mut maps = FkMaps::default();
        maps.ports.insert(live_port, closed_port);
        maps.ip_addresses.insert(live_ip, closed_ip);

        binding.remap_fks_for_clone(&maps);

        match binding.base.binding_type {
            BindingType::Port {
                port_id,
                ip_address_id,
            } => {
                assert_eq!(port_id, closed_port);
                assert_eq!(ip_address_id, Some(closed_ip));
            }
            other => panic!("unexpected binding type: {other:?}"),
        }
    }

    // FKs with no entry in the maps (e.g. a parent outside this site's
    // snapshot) must be left untouched rather than zeroed.
    #[test]
    fn unmapped_fks_are_left_unchanged() {
        let live_service = Uuid::new_v4();
        let live_ip = Uuid::new_v4();
        let mut binding = Binding::new(BindingBase::new(
            live_service,
            Uuid::new_v4(),
            BindingType::IPAddress {
                ip_address_id: live_ip,
            },
        ));

        binding.remap_fks_for_clone(&FkMaps::default());

        assert_eq!(binding.base.service_id, live_service);
        match binding.base.binding_type {
            BindingType::IPAddress { ip_address_id } => assert_eq!(ip_address_id, live_ip),
            other => panic!("unexpected binding type: {other:?}"),
        }
    }
}
