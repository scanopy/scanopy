use std::net::IpAddr;

use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use mac_address::MacAddress;
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    interfaces::r#impl::base::{Interface, InterfaceBase},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            child::ChildStorableEntity,
            traits::{Entity, SqlValue, Storable},
        },
    },
};

/// CSV row representation for Interface export
#[derive(Serialize)]
pub struct InterfaceCsvRow {
    pub id: Uuid,
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub name: Option<String>,
    pub host_id: Uuid,
    pub subnet_id: Uuid,
    pub network_id: Uuid,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Interface {
    type BaseData = InterfaceBase;

    fn table_name() -> &'static str {
        "interfaces"
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
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            base:
                Self::BaseData {
                    network_id,
                    host_id,
                    subnet_id,
                    ip_address,
                    mac_address,
                    name,
                    position,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "network_id",
                "host_id",
                "subnet_id",
                "ip_address",
                "mac_address",
                "name",
                "position",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Uuid(network_id),
                SqlValue::Uuid(host_id),
                SqlValue::Uuid(subnet_id),
                SqlValue::IpAddr(ip_address),
                SqlValue::OptionalMacAddress(mac_address),
                SqlValue::OptionalString(name),
                SqlValue::I32(position),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        // Read ip_address from INET column using IpNetwork
        let ip_network: IpNetwork = row
            .try_get("ip_address")
            .map_err(|e| anyhow::anyhow!("Failed to read ip_address: {}", e))?;
        let ip_address: IpAddr = ip_network.ip();

        // Read mac_address from MACADDR column
        let mac_address: Option<MacAddress> = row
            .try_get("mac_address")
            .map_err(|e| anyhow::anyhow!("Failed to read mac_address: {}", e))?;

        Ok(Interface {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: InterfaceBase {
                network_id: row.get("network_id"),
                host_id: row.get("host_id"),
                subnet_id: row.get("subnet_id"),
                ip_address,
                mac_address,
                name: row.get("name"),
                position: row.get("position"),
            },
        })
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

    type CsvRow = InterfaceCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        InterfaceCsvRow {
            id: self.id,
            ip_address: self.base.ip_address.to_string(),
            mac_address: self.base.mac_address.map(|m| m.to_string()),
            name: self.base.name.clone(),
            host_id: self.base.host_id,
            subnet_id: self.base.subnet_id,
            network_id: self.base.network_id,
            position: self.base.position,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Interface
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Interface";
    const ENTITY_NAME_PLURAL: &'static str = "Interfaces";
    const ENTITY_DESCRIPTION: &'static str = "Network interfaces on hosts. Each host can have multiple interfaces with different IP addresses.";

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

    fn preserve_immutable_fields(&mut self, existing: &Self) {
        self.created_at = existing.created_at;
        // MAC address is immutable once set
        if existing.base.mac_address.is_some() {
            self.base.mac_address = existing.base.mac_address;
        }
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
