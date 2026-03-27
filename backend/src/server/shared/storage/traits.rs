use std::net::IpAddr;

use crate::server::bindings::r#impl::base::Binding;
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::groups::r#impl::base::Group;
use crate::server::services::r#impl::base::Service;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::entity_metadata::EntityCategory;
use crate::server::shared::events::types::OnboardingOperation;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::tags::r#impl::base::Tag;
use crate::server::{
    billing::types::base::BillingPlan,
    daemons::r#impl::{api::DaemonCapabilities, base::DaemonMode},
    discovery::r#impl::types::{DiscoveryType, RunType},
    hosts::r#impl::{base::Host, virtualization::HostVirtualization},
    if_entries::r#impl::base::IfEntry,
    interfaces::r#impl::base::Interface,
    organizations::r#impl::base::PlanLimitNotifications,
    ports::r#impl::base::Port,
    services::r#impl::{definitions::ServiceDefinition, virtualization::ServiceVirtualization},
    shared::{storage::filter::StorableFilter, types::entities::EntitySource},
    topology::types::{
        base::TopologyOptions,
        edges::{Edge, EdgeStyle},
        nodes::Node,
    },
    users::r#impl::permissions::UserOrgPermissions,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cidr::IpCidr;
use email_address::EmailAddress;
use mac_address::MacAddress;
use sqlx::postgres::PgRow;
use stripe_billing::SubscriptionStatus;
use uuid::Uuid;

/// Result of a paginated query, containing items and total count.
#[derive(Debug, Clone)]
pub struct PaginatedResult<T> {
    /// The items for the current page
    pub items: Vec<T>,
    /// Total count of items matching the filter (ignoring limit/offset)
    pub total_count: u64,
}

#[async_trait]
pub trait Storage<T: Storable>: Send + Sync {
    async fn create(&self, entity: &T) -> Result<T, anyhow::Error>;
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<T>, anyhow::Error>;
    async fn get_all(&self, filter: StorableFilter<T>) -> Result<Vec<T>, anyhow::Error>;
    async fn get_all_ordered(
        &self,
        filter: StorableFilter<T>,
        order_by: &str,
    ) -> Result<Vec<T>, anyhow::Error>;
    /// Get entities with pagination, returning items and total count.
    /// The filter's limit/offset are applied to the query.
    async fn get_paginated(
        &self,
        filter: StorableFilter<T>,
        order_by: &str,
    ) -> Result<PaginatedResult<T>, anyhow::Error>;
    async fn get_one(&self, filter: StorableFilter<T>) -> Result<Option<T>, anyhow::Error>;
    async fn update(&self, entity: &mut T) -> Result<T, anyhow::Error>;
    async fn delete(&self, id: &Uuid) -> Result<(), anyhow::Error>;
    async fn create_many(&self, entities: &[T]) -> Result<Vec<T>, anyhow::Error>;
    async fn delete_many(&self, ids: &[Uuid]) -> Result<usize, anyhow::Error>;
    async fn delete_by_filter(&self, filter: StorableFilter<T>) -> Result<usize, anyhow::Error>;
}

/// Base trait for anything stored in the database, including junction tables.
/// Provides the minimal interface needed for storage operations.
pub trait Storable: Sized + Clone + Send + Sync + 'static + Default {
    type BaseData;

    fn new(base: Self::BaseData) -> Self;
    fn get_base(&self) -> Self::BaseData;

    /// Database table name
    fn table_name() -> &'static str;

    /// Serialization for database storage
    /// Returns (column_names, bind_values)
    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error>;

    /// Deserialization from database
    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error>;
}

/// Extended trait for user-facing domain entities (excludes junction tables).
/// Provides entity metadata, tenant scoping, timestamps, and tagging support.
pub trait Entity: Storable {
    /// Primary key
    fn id(&self) -> Uuid;
    fn created_at(&self) -> DateTime<Utc>;
    fn set_id(&mut self, id: Uuid);
    fn set_created_at(&mut self, time: DateTime<Utc>);

    /// Entity type discriminant for the entity enum
    fn entity_type() -> EntityDiscriminants;

    /// CSV row type for export. Must be Serialize.
    /// The csv crate derives headers automatically from field names.
    type CsvRow: serde::Serialize;

    /// Converts this entity to a CSV row struct.
    fn to_csv_row(&self) -> Self::CsvRow;

    /// Singular name for error messages (e.g., "host")
    /// Use the constant in const contexts, use the method at runtime.
    const ENTITY_NAME_SINGULAR: &'static str;

    /// Plural name for API paths and collections (e.g., "hosts")
    /// Use the constant in const contexts, use the method at runtime.
    const ENTITY_NAME_PLURAL: &'static str;

    /// Description for API documentation and database schema docs.
    /// Should be 1-3 sentences explaining the entity's purpose.
    const ENTITY_DESCRIPTION: &'static str;

    /// Category for documentation grouping.
    fn entity_category() -> EntityCategory;

    /// Singular name for error messages (e.g., "host")
    fn entity_name_singular() -> &'static str {
        Self::ENTITY_NAME_SINGULAR
    }

    /// Plural name for API paths and collections (e.g., "hosts")
    fn entity_name_plural() -> &'static str {
        Self::ENTITY_NAME_PLURAL
    }

    /// Tenant scoping - network context
    fn network_id(&self) -> Option<Uuid>;

    /// Tenant scoping - organization context
    fn organization_id(&self) -> Option<Uuid>;

    /// Whether entities of this type are scoped to a network
    fn is_network_keyed() -> bool {
        Self::default().network_id().is_some()
    }

    /// Whether entities of this type are scoped to an organization
    fn is_organization_keyed() -> bool {
        Self::default().organization_id().is_some()
    }

    /// Last modification timestamp
    fn updated_at(&self) -> DateTime<Utc>;
    fn set_updated_at(&mut self, time: DateTime<Utc>);

    /// Whether this entity type supports tagging.
    /// Default implementation delegates to is_entity_taggable().
    fn is_taggable() -> bool {
        crate::server::shared::entities::is_entity_taggable(Self::entity_type())
    }

    /// Get the tags field from the entity for validation.
    /// Override for entities with a tags field.
    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        None
    }

    /// Set the tags field on the entity.
    /// Override for entities with a tags field.
    fn set_tags(&mut self, _tags: Vec<Uuid>) {
        // Default: no-op
    }

    /// Set the source field on the entity.
    /// Override for entities with a source field.
    fn set_source(&mut self, _source: EntitySource) {
        // Default: no-op
    }

    /// Preserve entity-specific immutable fields from the existing entity.
    /// Override for entities that have additional read-only fields beyond id/created_at.
    fn preserve_immutable_fields(&mut self, _existing: &Self) {
        // Default: no-op
    }
}

/// Helper type for SQL values
#[derive(Clone)]
pub enum SqlValue {
    Uuid(Uuid),
    OptionalUuid(Option<Uuid>),
    String(String),
    OptionalString(Option<String>),
    I32(i32),
    OptionalI64(Option<i64>),
    U16(u16),
    Bool(bool),
    Email(EmailAddress),
    Timestamp(DateTime<Utc>),
    OptionTimestamp(Option<DateTime<Utc>>),
    UuidArray(Vec<Uuid>),
    IpCidr(IpCidr),
    IpAddr(IpAddr),
    OptionalIpAddr(Option<IpAddr>),
    EntitySource(EntitySource),
    EntityDiscriminant(EntityDiscriminants),
    ServiceDefinition(Box<dyn ServiceDefinition>),
    OptionalServiceVirtualization(Option<ServiceVirtualization>),
    OptionalHostVirtualization(Option<HostVirtualization>),
    Ports(Vec<Port>),
    Interfaces(Vec<Interface>),
    RunType(RunType),
    DiscoveryType(DiscoveryType),
    DaemonCapabilities(DaemonCapabilities),
    UserOrgPermissions(UserOrgPermissions),
    OptionBillingPlan(Option<BillingPlan>),
    OptionBillingPlanStatus(Option<SubscriptionStatus>),
    EdgeStyle(EdgeStyle),
    DaemonMode(DaemonMode),
    Nodes(Vec<Node>),
    Edges(Vec<Edge>),
    TopologyOptions(TopologyOptions),
    Hosts(Vec<Host>),
    Subnets(Vec<Subnet>),
    Services(Vec<Service>),
    Bindings(Vec<Binding>),
    Groups(Vec<Group>),
    OnboardingOperation(Vec<OnboardingOperation>),
    StringArray(Vec<String>),
    OptionalStringArray(Option<Vec<String>>),
    OptionalLldpChassisId(Option<crate::server::snmp::resolution::lldp::LldpChassisId>),
    OptionalLldpPortId(Option<crate::server::snmp::resolution::lldp::LldpPortId>),
    OptionalFdbMacs(Option<Vec<String>>),
    ShareOptions(crate::server::shares::r#impl::base::ShareOptions),
    CredentialType(CredentialType),
    MacAddress(MacAddress),
    OptionalMacAddress(Option<MacAddress>),
    IfEntries(Vec<IfEntry>),
    Tags(Vec<Tag>),
    PlanLimitNotifications(PlanLimitNotifications),
    OptionalIpAddrArray(Option<Vec<IpAddr>>),
    OptionalUuidVec(Option<Vec<Uuid>>),
}
