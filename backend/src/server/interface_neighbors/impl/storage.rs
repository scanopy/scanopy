use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::{
    interface_neighbors::r#impl::base::{
        InterfaceNeighborCandidate, InterfaceNeighborCandidateBase, InterfaceNeighborEvidence,
        InterfaceNeighborHost, InterfaceNeighborHostBase, InterfaceNeighborInterface,
        InterfaceNeighborInterfaceBase,
    },
    lldp::{LldpChassisId, LldpPortId},
    shared::storage::{
        snapshot::{FkMaps, Snapshotable},
        traits::{SqlValue, Storable},
    },
};

fn parse_lldp_chassis_id(row: &PgRow, column: &str) -> Option<LldpChassisId> {
    let value: Option<serde_json::Value> = row.get(column);
    value
        .filter(|v| !v.is_null())
        .and_then(|v| serde_json::from_value(v).ok())
}

fn parse_lldp_port_id(row: &PgRow, column: &str) -> Option<LldpPortId> {
    let value: Option<serde_json::Value> = row.get(column);
    value
        .filter(|v| !v.is_null())
        .and_then(|v| serde_json::from_value(v).ok())
}

// ============================================================================
// InterfaceNeighborCandidate
// ============================================================================

impl Storable for InterfaceNeighborCandidate {
    const HAS_SCD2: bool = false;
    type BaseData = InterfaceNeighborCandidateBase;

    fn table_name() -> &'static str {
        "interface_neighbor_candidates"
    }

    fn new(base: Self::BaseData) -> Self {
        InterfaceNeighborCandidate::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let evidence = &self.base.evidence;
        Ok((
            vec![
                "id",
                "network_id",
                "interface_id",
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
                "created_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.network_id),
                SqlValue::Uuid(self.base.interface_id),
                SqlValue::OptionalLldpChassisId(evidence.lldp_chassis_id.clone()),
                SqlValue::OptionalLldpPortId(evidence.lldp_port_id.clone()),
                SqlValue::OptionalString(evidence.lldp_sys_name.clone()),
                SqlValue::OptionalString(evidence.lldp_port_desc.clone()),
                SqlValue::OptionalIpAddr(evidence.lldp_mgmt_addr),
                SqlValue::OptionalString(evidence.lldp_sys_desc.clone()),
                SqlValue::OptionalString(evidence.cdp_device_id.clone()),
                SqlValue::OptionalString(evidence.cdp_port_id.clone()),
                SqlValue::OptionalString(evidence.cdp_platform.clone()),
                SqlValue::OptionalIpAddr(evidence.cdp_address),
                SqlValue::Timestamp(self.created_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(InterfaceNeighborCandidate {
            id: row.get("id"),
            created_at: row.get("created_at"),
            base: InterfaceNeighborCandidateBase {
                network_id: row.get("network_id"),
                interface_id: row.get("interface_id"),
                evidence: InterfaceNeighborEvidence {
                    lldp_chassis_id: parse_lldp_chassis_id(row, "lldp_chassis_id"),
                    lldp_port_id: parse_lldp_port_id(row, "lldp_port_id"),
                    lldp_sys_name: row.get("lldp_sys_name"),
                    lldp_port_desc: row.get("lldp_port_desc"),
                    lldp_mgmt_addr: row.try_get("lldp_mgmt_addr").ok().flatten(),
                    lldp_sys_desc: row.get("lldp_sys_desc"),
                    cdp_device_id: row.get("cdp_device_id"),
                    cdp_port_id: row.get("cdp_port_id"),
                    cdp_platform: row.get("cdp_platform"),
                    cdp_address: row.try_get("cdp_address").ok().flatten(),
                },
            },
        })
    }
}

// ============================================================================
// InterfaceNeighborInterface
// ============================================================================

impl Storable for InterfaceNeighborInterface {
    type BaseData = InterfaceNeighborInterfaceBase;

    const HAS_SCD2: bool = true;

    fn is_live_row(&self) -> bool {
        self.valid_to.is_none()
    }

    fn table_name() -> &'static str {
        "interface_neighbor_interfaces"
    }

    fn new(base: Self::BaseData) -> Self {
        InterfaceNeighborInterface::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        Ok((
            vec![
                "id",
                "network_id",
                "interface_id",
                "neighbor_interface_id",
                "neighbor_seen_at",
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
                SqlValue::Uuid(self.base.network_id),
                SqlValue::Uuid(self.base.interface_id),
                SqlValue::Uuid(self.base.neighbor_interface_id),
                SqlValue::OptionTimestamp(self.base.neighbor_seen_at),
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
        Ok(InterfaceNeighborInterface {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            last_seen_at: row.get("last_seen_at"),
            last_discovery_id: row.get("last_discovery_id"),
            first_discovery_id: row.get("first_discovery_id"),
            base: InterfaceNeighborInterfaceBase {
                network_id: row.get("network_id"),
                interface_id: row.get("interface_id"),
                neighbor_interface_id: row.get("neighbor_interface_id"),
                neighbor_seen_at: row.get("neighbor_seen_at"),
            },
        })
    }
}

impl Snapshotable for InterfaceNeighborInterface {
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

    /// Both FKs point at `interfaces`, cloned earlier in `CLONE_ORDER` — unlike `Interface`'s own
    /// former `neighbor_interface_id`, neither is a *self*-reference (this table, not
    /// `interfaces`, is what carries the pair), so both remap in this single pass with no
    /// two-phase `own_clone_ref` dance.
    fn remap_fks_for_clone(&mut self, maps: &FkMaps) {
        if let Some(closed) = maps.interfaces.get(&self.base.interface_id) {
            self.base.interface_id = *closed;
        }
        if let Some(closed) = maps.interfaces.get(&self.base.neighbor_interface_id) {
            self.base.neighbor_interface_id = *closed;
        }
    }
}

// ============================================================================
// InterfaceNeighborHost
// ============================================================================

impl Storable for InterfaceNeighborHost {
    type BaseData = InterfaceNeighborHostBase;

    const HAS_SCD2: bool = true;

    fn is_live_row(&self) -> bool {
        self.valid_to.is_none()
    }

    fn table_name() -> &'static str {
        "interface_neighbor_hosts"
    }

    fn new(base: Self::BaseData) -> Self {
        InterfaceNeighborHost::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        Ok((
            vec![
                "id",
                "network_id",
                "interface_id",
                "neighbor_host_id",
                "neighbor_seen_at",
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
                SqlValue::Uuid(self.base.network_id),
                SqlValue::Uuid(self.base.interface_id),
                SqlValue::Uuid(self.base.neighbor_host_id),
                SqlValue::OptionTimestamp(self.base.neighbor_seen_at),
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
        Ok(InterfaceNeighborHost {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            valid_from: row.get("valid_from"),
            valid_to: row.get("valid_to"),
            lineage_id: row.get("lineage_id"),
            last_seen_at: row.get("last_seen_at"),
            last_discovery_id: row.get("last_discovery_id"),
            first_discovery_id: row.get("first_discovery_id"),
            base: InterfaceNeighborHostBase {
                network_id: row.get("network_id"),
                interface_id: row.get("interface_id"),
                neighbor_host_id: row.get("neighbor_host_id"),
                neighbor_seen_at: row.get("neighbor_seen_at"),
            },
        })
    }
}

impl Snapshotable for InterfaceNeighborHost {
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
        if let Some(closed) = maps.interfaces.get(&self.base.interface_id) {
            self.base.interface_id = *closed;
        }
        if let Some(closed) = maps.hosts.get(&self.base.neighbor_host_id) {
            self.base.neighbor_host_id = *closed;
        }
    }
}
