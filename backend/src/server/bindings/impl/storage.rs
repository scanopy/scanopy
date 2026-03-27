use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

use crate::server::{
    bindings::r#impl::base::{Binding, BindingBase, BindingType},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
};

/// CSV row representation for Binding export
#[derive(Serialize)]
pub struct BindingCsvRow {
    pub id: Uuid,
    pub service_id: Uuid,
    pub binding_type: String,
    pub interface_id: Option<Uuid>,
    pub port_id: Option<Uuid>,
    pub network_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Binding {
    type BaseData = BindingBase;

    fn table_name() -> &'static str {
        "bindings"
    }

    fn new(base: Self::BaseData) -> Self {
        Binding::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let (binding_type, interface_id, port_id) = match self.base.binding_type {
            BindingType::Interface { interface_id } => ("Interface", Some(interface_id), None),
            BindingType::Port {
                port_id,
                interface_id,
            } => ("Port", interface_id, Some(port_id)),
        };

        Ok((
            vec![
                "id",
                "service_id",
                "network_id",
                "binding_type",
                "interface_id",
                "port_id",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.service_id),
                SqlValue::Uuid(self.base.network_id),
                SqlValue::String(binding_type.to_string()),
                SqlValue::OptionalUuid(interface_id),
                SqlValue::OptionalUuid(port_id),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let id: Uuid = row.get("id");
        let service_id: Uuid = row.get("service_id");
        let network_id: Uuid = row.get("network_id");
        let created_at: DateTime<Utc> = row.get("created_at");
        let updated_at: DateTime<Utc> = row.get("updated_at");
        let binding_type_str: String = row.get("binding_type");
        let interface_id: Option<Uuid> = row.get("interface_id");
        let port_id: Option<Uuid> = row.get("port_id");

        let binding_type = match binding_type_str.as_str() {
            "Interface" => {
                let interface_id = interface_id
                    .ok_or_else(|| anyhow::anyhow!("Interface binding missing interface_id"))?;
                BindingType::Interface { interface_id }
            }
            "Port" => {
                let port_id =
                    port_id.ok_or_else(|| anyhow::anyhow!("Port binding missing port_id"))?;
                BindingType::Port {
                    port_id,
                    interface_id, // Can be None for "all interfaces"
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
            base: BindingBase {
                service_id,
                network_id,
                binding_type,
            },
        })
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
        let (binding_type, interface_id, port_id) = match self.base.binding_type {
            BindingType::Interface { interface_id } => ("Interface", Some(interface_id), None),
            BindingType::Port {
                port_id,
                interface_id,
            } => ("Port", interface_id, Some(port_id)),
        };
        BindingCsvRow {
            id: self.id,
            service_id: self.base.service_id,
            binding_type: binding_type.to_string(),
            interface_id,
            port_id,
            network_id: self.base.network_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Binding
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Binding";
    const ENTITY_NAME_PLURAL: &'static str = "Bindings";
    const ENTITY_DESCRIPTION: &'static str = "Service bindings linking services to interfaces and/or ports. Defines where a service is accessible.";

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
