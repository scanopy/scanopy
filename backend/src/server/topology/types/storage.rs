use crate::server::bindings::r#impl::base::Binding;
use crate::server::groups::r#impl::base::Group;
use crate::server::if_entries::r#impl::base::IfEntry;
use crate::server::interfaces::r#impl::base::Interface;
use crate::server::ports::r#impl::base::Port;
use crate::server::services::r#impl::base::Service;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::tags::r#impl::base::Tag;
use crate::server::{
    hosts::r#impl::base::Host,
    shared::{
        entity_metadata::EntityCategory,
        storage::traits::{Entity, SqlValue, Storable},
    },
    topology::types::{
        base::{Topology, TopologyBase, TopologyOptions},
        edges::Edge,
        nodes::Node,
    },
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

/// CSV row representation for Topology export (metadata only, excludes nested entities)
#[derive(Serialize)]
pub struct TopologyCsvRow {
    pub id: Uuid,
    pub name: String,
    pub network_id: Uuid,
    pub is_stale: bool,
    pub is_locked: bool,
    pub locked_by: Option<Uuid>,
    pub locked_at: Option<DateTime<Utc>>,
    pub last_refreshed: DateTime<Utc>,
    pub parent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Storable for Topology {
    type BaseData = TopologyBase;

    fn table_name() -> &'static str {
        "topologies"
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
                    name,
                    network_id,
                    nodes,
                    edges,
                    options,
                    hosts,
                    interfaces,
                    ports,
                    bindings,
                    services,
                    subnets,
                    groups,
                    if_entries,
                    is_stale,
                    last_refreshed,
                    is_locked,
                    locked_at,
                    locked_by,
                    removed_hosts,
                    removed_interfaces,
                    removed_services,
                    removed_subnets,
                    removed_groups,
                    removed_bindings,
                    removed_ports,
                    removed_if_entries,
                    parent_id,
                    tags,
                    entity_tags,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "name",
                "network_id",
                "nodes",
                "edges",
                "options",
                "hosts",
                "interfaces",
                "subnets",
                "groups",
                "services",
                "bindings",
                "ports",
                "if_entries",
                "is_stale",
                "last_refreshed",
                "is_locked",
                "locked_at",
                "locked_by",
                "removed_hosts",
                "removed_interfaces",
                "removed_services",
                "removed_subnets",
                "removed_groups",
                "removed_bindings",
                "removed_ports",
                "removed_if_entries",
                "parent_id",
                "tags",
                "entity_tags",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(name),
                SqlValue::Uuid(network_id),
                SqlValue::Nodes(nodes),
                SqlValue::Edges(edges),
                SqlValue::TopologyOptions(options),
                SqlValue::Hosts(hosts),
                SqlValue::Interfaces(interfaces),
                SqlValue::Subnets(subnets),
                SqlValue::Groups(groups),
                SqlValue::Services(services),
                SqlValue::Bindings(bindings),
                SqlValue::Ports(ports),
                SqlValue::IfEntries(if_entries),
                SqlValue::Bool(is_stale),
                SqlValue::Timestamp(last_refreshed),
                SqlValue::Bool(is_locked),
                SqlValue::OptionTimestamp(locked_at),
                SqlValue::OptionalUuid(locked_by),
                SqlValue::UuidArray(removed_hosts),
                SqlValue::UuidArray(removed_interfaces),
                SqlValue::UuidArray(removed_services),
                SqlValue::UuidArray(removed_subnets),
                SqlValue::UuidArray(removed_groups),
                SqlValue::UuidArray(removed_bindings),
                SqlValue::UuidArray(removed_ports),
                SqlValue::UuidArray(removed_if_entries),
                SqlValue::OptionalUuid(parent_id),
                SqlValue::UuidArray(tags),
                SqlValue::Tags(entity_tags),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        // Parse JSON fields safely
        let nodes: Vec<Node> = serde_json::from_value(row.get::<serde_json::Value, _>("nodes"))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize nodes: {}", e))?;
        let edges: Vec<Edge> = serde_json::from_value(row.get::<serde_json::Value, _>("edges"))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize edges: {}", e))?;
        let options: TopologyOptions =
            serde_json::from_value(row.get::<serde_json::Value, _>("options"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize options: {}", e))?;

        let hosts: Vec<Host> = serde_json::from_value(row.get::<serde_json::Value, _>("hosts"))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize hosts: {}", e))?;
        let interfaces: Vec<Interface> =
            serde_json::from_value(row.get::<serde_json::Value, _>("interfaces"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize interfaces: {}", e))?;
        let subnets: Vec<Subnet> =
            serde_json::from_value(row.get::<serde_json::Value, _>("subnets"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize subnets: {}", e))?;
        let services: Vec<Service> =
            serde_json::from_value(row.get::<serde_json::Value, _>("services"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize services: {}", e))?;
        let groups: Vec<Group> = serde_json::from_value(row.get::<serde_json::Value, _>("groups"))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize groups: {}", e))?;

        let ports: Vec<Port> = serde_json::from_value(row.get::<serde_json::Value, _>("ports"))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize ports: {}", e))?;

        let bindings: Vec<Binding> =
            serde_json::from_value(row.get::<serde_json::Value, _>("bindings"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize bindings: {}", e))?;

        let if_entries: Vec<IfEntry> =
            serde_json::from_value(row.get::<serde_json::Value, _>("if_entries"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize if_entries: {}", e))?;

        let entity_tags: Vec<Tag> =
            serde_json::from_value(row.get::<serde_json::Value, _>("entity_tags"))
                .map_err(|e| anyhow::anyhow!("Failed to deserialize entity_tags: {}", e))?;

        Ok(Topology {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: TopologyBase {
                name: row.get("name"),
                network_id: row.get("network_id"),
                is_stale: row.get("is_stale"),
                last_refreshed: row.get("last_refreshed"),
                is_locked: row.get("is_locked"),
                locked_at: row.get("locked_at"),
                locked_by: row.get("locked_by"),
                removed_groups: row.get("removed_groups"),
                removed_hosts: row.get("removed_hosts"),
                removed_interfaces: row.get("removed_interfaces"),
                removed_services: row.get("removed_services"),
                removed_subnets: row.get("removed_subnets"),
                removed_ports: row.get("removed_ports"),
                removed_bindings: row.get("removed_bindings"),
                removed_if_entries: row.get("removed_if_entries"),
                parent_id: row.get("parent_id"),
                nodes,
                edges,
                hosts,
                interfaces,
                subnets,
                bindings,
                ports,
                services,
                groups,
                if_entries,
                options,
                tags: row.get("tags"),
                entity_tags,
            },
        })
    }
}

impl Entity for Topology {
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

    type CsvRow = TopologyCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        TopologyCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            network_id: self.base.network_id,
            is_stale: self.base.is_stale,
            is_locked: self.base.is_locked,
            locked_by: self.base.locked_by,
            locked_at: self.base.locked_at,
            last_refreshed: self.base.last_refreshed,
            parent_id: self.base.parent_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Topology
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Topology";
    const ENTITY_NAME_PLURAL: &'static str = "Topologies";
    const ENTITY_DESCRIPTION: &'static str =
        "Network topology maps showing host relationships and connections.";

    fn entity_category() -> EntityCategory {
        EntityCategory::Visualization
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
        self.id = existing.id;
        self.base.parent_id = existing.base.parent_id;
        self.created_at = existing.created_at;
        self.updated_at = existing.updated_at;
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }
}
