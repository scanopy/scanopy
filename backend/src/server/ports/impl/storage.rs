use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    ports::r#impl::base::{Port, PortBase, PortConfig, PortType, TransportProtocol},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            snapshot::{DiscoveryTracked, FkMaps, Snapshotable},
            traits::{Entity, SqlValue, Storable},
        },
    },
};

/// CSV row representation for Port export
#[derive(Serialize)]
pub struct PortCsvRow {
    pub id: Uuid,
    pub port_number: u16,
    pub protocol: String,
    pub port_type: String,
    pub host_id: Uuid,
    pub site_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Port {
    type BaseData = PortBase;

    fn table_name() -> &'static str {
        "ports"
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
            base,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        self.base
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let config = self.base.port_type.config();
        let port_type = Self::port_type_string(&self.base.port_type);
        let protocol = Self::protocol_string(config.protocol);

        Ok((
            vec![
                "id",
                "host_id",
                "site_id",
                "port_number",
                "protocol",
                "port_type",
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
                SqlValue::Uuid(self.base.host_id),
                SqlValue::Uuid(self.base.site_id),
                SqlValue::I32(config.number as i32),
                SqlValue::String(protocol.to_string()),
                SqlValue::String(port_type),
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
        let host_id: Uuid = row.get("host_id");
        let site_id: Uuid = row.get("site_id");
        let created_at: DateTime<Utc> = row.get("created_at");
        let updated_at: DateTime<Utc> = row.get("updated_at");
        let port_number: i32 = row.get("port_number");
        let protocol: String = row.get("protocol");

        let protocol = match protocol.as_str() {
            "Tcp" => TransportProtocol::Tcp,
            "Udp" => TransportProtocol::Udp,
            _ => TransportProtocol::Tcp, // Default fallback
        };

        // Try to find a matching predefined port variant
        use strum::IntoEnumIterator;
        let port_type = PortType::iter()
            .find(|variant| {
                if matches!(variant, PortType::Custom(_)) {
                    return false;
                }
                let config = variant.config();
                config.number == port_number as u16 && config.protocol == protocol
            })
            .unwrap_or(PortType::Custom(PortConfig {
                number: port_number as u16,
                protocol,
            }));

        Ok(Port {
            id,
            created_at,
            updated_at,
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            last_seen_at: row.get("last_seen_at"),
            last_discovery_id: row.get("last_discovery_id"),
            first_discovery_id: row.get("first_discovery_id"),
            base: PortBase {
                host_id,
                site_id,
                port_type,
            },
        })
    }
}

impl Snapshotable for Port {
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

impl DiscoveryTracked for Port {
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
            &scanned.port_ids,
        )
    }
}

impl Entity for Port {
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

    type CsvRow = PortCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        let config = self.base.port_type.config();
        PortCsvRow {
            id: self.id,
            port_number: config.number,
            protocol: format!("{:?}", config.protocol),
            port_type: Self::port_type_string(&self.base.port_type),
            host_id: self.base.host_id,
            site_id: self.base.site_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Port
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Port";
    const ENTITY_NAME_PLURAL: &'static str = "Ports";
    const ENTITY_DESCRIPTION: &'static str =
        "Ports that have been scanned and found open on a host.";

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

impl Port {
    fn protocol_string(protocol: TransportProtocol) -> &'static str {
        match protocol {
            TransportProtocol::Tcp => "Tcp",
            TransportProtocol::Udp => "Udp",
        }
    }

    fn port_type_string(port_type: &PortType) -> String {
        match port_type {
            PortType::Custom(_) => "Custom".to_string(),
            _ => {
                let s: &'static str = port_type.into();
                s.to_string()
            }
        }
    }
}
