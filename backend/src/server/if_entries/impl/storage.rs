use chrono::{DateTime, Utc};
use mac_address::MacAddress;
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    if_entries::r#impl::base::{IfAdminStatus, IfEntry, IfEntryBase, IfOperStatus, Neighbor},
    shared::{
        entities::EntityDiscriminants,
        entity_metadata::EntityCategory,
        storage::{
            child::ChildStorableEntity,
            traits::{Entity, SqlValue, Storable},
        },
    },
};

/// CSV row representation for IfEntry export
#[derive(Serialize)]
pub struct IfEntryCsvRow {
    pub id: Uuid,
    pub host_id: Uuid,
    pub network_id: Uuid,
    pub if_index: i32,
    pub if_descr: String,
    pub if_name: Option<String>,
    pub if_alias: Option<String>,
    pub if_type: i32,
    pub speed_bps: Option<i64>,
    pub admin_status: String,
    pub oper_status: String,
    pub mac_address: Option<String>,
    pub interface_id: Option<Uuid>,
    pub neighbor: Option<String>,
    pub lldp_chassis_id: Option<String>,
    pub lldp_port_id: Option<String>,
    pub lldp_sys_name: Option<String>,
    pub lldp_port_desc: Option<String>,
    pub lldp_mgmt_addr: Option<String>,
    pub lldp_sys_desc: Option<String>,
    pub cdp_device_id: Option<String>,
    pub cdp_port_id: Option<String>,
    pub cdp_platform: Option<String>,
    pub cdp_address: Option<String>,
    pub fdb_macs: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for IfEntry {
    type BaseData = IfEntryBase;

    fn table_name() -> &'static str {
        "if_entries"
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
                    host_id,
                    network_id,
                    if_index,
                    if_descr,
                    if_name,
                    if_alias,
                    if_type,
                    speed_bps,
                    admin_status,
                    oper_status,
                    mac_address,
                    interface_id,
                    neighbor,
                    lldp_chassis_id,
                    lldp_port_id,
                    lldp_sys_name,
                    lldp_port_desc,
                    lldp_mgmt_addr,
                    lldp_sys_desc,
                    cdp_device_id,
                    cdp_port_id,
                    cdp_platform,
                    cdp_address,
                    fdb_macs,
                },
        } = self.clone();

        // Convert Neighbor enum to two mutually exclusive columns
        let (neighbor_if_entry_id, neighbor_host_id) = match neighbor {
            Some(Neighbor::IfEntry(id)) => (Some(id), None),
            Some(Neighbor::Host(id)) => (None, Some(id)),
            None => (None, None),
        };

        let columns = vec![
            "id",
            "host_id",
            "network_id",
            "if_index",
            "if_descr",
            "if_name",
            "if_alias",
            "if_type",
            "speed_bps",
            "admin_status",
            "oper_status",
            "mac_address",
            "interface_id",
            "neighbor_if_entry_id",
            "neighbor_host_id",
            "lldp_chassis_id",
            "lldp_port_id",
            "lldp_sys_name",
            "lldp_port_desc",
            "lldp_mgmt_addr",
            "lldp_sys_desc",
            "cdp_device_id",
            "cdp_port_id",
            "cdp_platform",
            "cdp_address",
            "fdb_macs",
            "created_at",
            "updated_at",
        ];

        let values = vec![
            SqlValue::Uuid(id),
            SqlValue::Uuid(host_id),
            SqlValue::Uuid(network_id),
            SqlValue::I32(if_index),
            SqlValue::String(if_descr),
            SqlValue::OptionalString(if_name),
            SqlValue::OptionalString(if_alias),
            SqlValue::I32(if_type),
            SqlValue::OptionalI64(speed_bps),
            SqlValue::I32(i32::from(admin_status)),
            SqlValue::I32(i32::from(oper_status)),
            SqlValue::OptionalMacAddress(mac_address),
            SqlValue::OptionalUuid(interface_id),
            SqlValue::OptionalUuid(neighbor_if_entry_id),
            SqlValue::OptionalUuid(neighbor_host_id),
            SqlValue::OptionalLldpChassisId(lldp_chassis_id),
            SqlValue::OptionalLldpPortId(lldp_port_id),
            SqlValue::OptionalString(lldp_sys_name),
            SqlValue::OptionalString(lldp_port_desc),
            SqlValue::OptionalIpAddr(lldp_mgmt_addr),
            SqlValue::OptionalString(lldp_sys_desc),
            SqlValue::OptionalString(cdp_device_id),
            SqlValue::OptionalString(cdp_port_id),
            SqlValue::OptionalString(cdp_platform),
            SqlValue::OptionalIpAddr(cdp_address),
            SqlValue::OptionalFdbMacs(fdb_macs),
            SqlValue::Timestamp(created_at),
            SqlValue::Timestamp(updated_at),
        ];

        Ok((columns, values))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        use crate::server::snmp::resolution::lldp::{LldpChassisId, LldpPortId};

        let admin_status_raw: i32 = row.get("admin_status");
        let oper_status_raw: i32 = row.get("oper_status");

        // Handle speed_bps which might be NULL or a large value
        let speed_bps: Option<i64> = row.get("speed_bps");

        // Read mac_address from MACADDR column
        let mac_address: Option<MacAddress> = row
            .try_get("mac_address")
            .map_err(|e| anyhow::anyhow!("Failed to read mac_address: {}", e))?;

        // Parse neighbor columns into Neighbor enum
        let neighbor_if_entry_id: Option<Uuid> = row.get("neighbor_if_entry_id");
        let neighbor_host_id: Option<Uuid> = row.get("neighbor_host_id");
        let neighbor = match (neighbor_if_entry_id, neighbor_host_id) {
            (Some(id), None) => Some(Neighbor::IfEntry(id)),
            (None, Some(id)) => Some(Neighbor::Host(id)),
            (None, None) => None,
            // DB constraint should prevent this, but handle gracefully
            (Some(_), Some(_)) => {
                tracing::warn!(
                    "IfEntry has both neighbor_if_entry_id and neighbor_host_id set, using neighbor_if_entry_id"
                );
                Some(Neighbor::IfEntry(neighbor_if_entry_id.unwrap()))
            }
        };

        // Parse LLDP JSON fields - they may be null
        let lldp_chassis_json: Option<serde_json::Value> = row.get("lldp_chassis_id");
        let lldp_chassis_id: Option<LldpChassisId> = lldp_chassis_json.and_then(|v| {
            if v.is_null() {
                None
            } else {
                serde_json::from_value(v).ok()
            }
        });

        let lldp_port_json: Option<serde_json::Value> = row.get("lldp_port_id");
        let lldp_port_id: Option<LldpPortId> = lldp_port_json.and_then(|v| {
            if v.is_null() {
                None
            } else {
                serde_json::from_value(v).ok()
            }
        });

        Ok(IfEntry {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: IfEntryBase {
                host_id: row.get("host_id"),
                network_id: row.get("network_id"),
                if_index: row.get("if_index"),
                if_descr: row.get("if_descr"),
                if_name: row.get("if_name"),
                if_alias: row.get("if_alias"),
                if_type: row.get("if_type"),
                speed_bps,
                admin_status: IfAdminStatus::from(admin_status_raw),
                oper_status: IfOperStatus::from(oper_status_raw),
                mac_address,
                interface_id: row.get("interface_id"),
                neighbor,
                lldp_chassis_id,
                lldp_port_id,
                lldp_sys_name: row.get("lldp_sys_name"),
                lldp_port_desc: row.get("lldp_port_desc"),
                lldp_mgmt_addr: row.try_get("lldp_mgmt_addr").ok().flatten(),
                lldp_sys_desc: row.get("lldp_sys_desc"),
                cdp_device_id: row.get("cdp_device_id"),
                cdp_port_id: row.get("cdp_port_id"),
                cdp_platform: row.get("cdp_platform"),
                cdp_address: row.try_get("cdp_address").ok().flatten(),
                fdb_macs: row
                    .try_get::<Option<serde_json::Value>, _>("fdb_macs")
                    .ok()
                    .flatten()
                    .and_then(|v| serde_json::from_value(v).ok()),
            },
        })
    }
}

impl Entity for IfEntry {
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

    type CsvRow = IfEntryCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        IfEntryCsvRow {
            id: self.id,
            host_id: self.base.host_id,
            network_id: self.base.network_id,
            if_index: self.base.if_index,
            if_descr: self.base.if_descr.clone(),
            if_name: self.base.if_name.clone(),
            if_alias: self.base.if_alias.clone(),
            if_type: self.base.if_type,
            speed_bps: self.base.speed_bps,
            admin_status: format!("{:?}", self.base.admin_status),
            oper_status: format!("{:?}", self.base.oper_status),
            mac_address: self.base.mac_address.map(|m| m.to_string()),
            interface_id: self.base.interface_id,
            neighbor: self.base.neighbor.as_ref().map(|n| match n {
                Neighbor::IfEntry(id) => format!("IfEntry:{}", id),
                Neighbor::Host(id) => format!("Host:{}", id),
            }),
            lldp_chassis_id: self
                .base
                .lldp_chassis_id
                .as_ref()
                .and_then(|c| serde_json::to_string(c).ok()),
            lldp_port_id: self
                .base
                .lldp_port_id
                .as_ref()
                .and_then(|p| serde_json::to_string(p).ok()),
            lldp_sys_name: self.base.lldp_sys_name.clone(),
            lldp_port_desc: self.base.lldp_port_desc.clone(),
            lldp_mgmt_addr: self.base.lldp_mgmt_addr.map(|a| a.to_string()),
            lldp_sys_desc: self.base.lldp_sys_desc.clone(),
            cdp_device_id: self.base.cdp_device_id.clone(),
            cdp_port_id: self.base.cdp_port_id.clone(),
            cdp_platform: self.base.cdp_platform.clone(),
            cdp_address: self.base.cdp_address.map(|a| a.to_string()),
            fdb_macs: self
                .base
                .fdb_macs
                .as_ref()
                .and_then(|m| serde_json::to_string(m).ok()),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::IfEntry
    }

    const ENTITY_NAME_SINGULAR: &'static str = "ifTable Entry";
    const ENTITY_NAME_PLURAL: &'static str = "ifTable Entries";
    const ENTITY_DESCRIPTION: &'static str = "SNMP interface entries (ifTable). Physical and logical interfaces discovered via SNMP on hosts.";

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
        // MAC address is immutable once set (from SNMP ifPhysAddress)
        if existing.base.mac_address.is_some() {
            self.base.mac_address = existing.base.mac_address;
        }
    }
}

impl ChildStorableEntity for IfEntry {
    fn parent_column() -> &'static str {
        "host_id"
    }

    fn parent_id(&self) -> Uuid {
        self.base.host_id
    }
}
