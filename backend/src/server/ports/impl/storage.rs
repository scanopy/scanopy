use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    ports::r#impl::base::{Port, PortBase, PortConfig, PortType, TransportProtocol},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
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
    pub network_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Port {
    type BaseData = PortBase;

    fn table_name() -> &'static str {
        "ports"
    }

    fn new(base: Self::BaseData) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
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
                "network_id",
                "port_number",
                "protocol",
                "port_type",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.host_id),
                SqlValue::Uuid(self.base.network_id),
                SqlValue::I32(config.number as i32),
                SqlValue::String(protocol.to_string()),
                SqlValue::String(port_type),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let id: Uuid = row.get("id");
        let host_id: Uuid = row.get("host_id");
        let network_id: Uuid = row.get("network_id");
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
            base: PortBase {
                host_id,
                network_id,
                port_type,
            },
        })
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
            network_id: self.base.network_id,
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
