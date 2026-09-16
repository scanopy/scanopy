use std::net::IpAddr;

use crate::daemon::discovery::types::warnings::{
    ClaimSource, DiscoveryWarningCode, MalformedNeighbourConsequence, SnmpWalkGroup,
};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::bindings::r#impl::base::Binding;
use crate::server::credentials::r#impl::mapping::{
    CredentialQueryPayloadDiscriminants, IntegrationTarget, Target,
};
use crate::server::credentials::r#impl::types::CredentialType;
use crate::server::dependencies::r#impl::base::Dependency;
use crate::server::lldp::{LldpChassisId, LldpPortId};
use crate::server::services::r#impl::base::Service;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeSource, AttributeSourceDiscriminants};
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::entity_metadata::EntityCategory;
use crate::server::shared::events::types::{BillingOperation, OnboardingOperationDiscriminants};
use crate::server::shares::r#impl::base::ShareOptions;
use crate::server::subnets::r#impl::base::Subnet;
use crate::server::tags::r#impl::base::Tag;
use crate::server::topology::types::views::TopologyView;
use crate::server::vlans::r#impl::base::Vlan;
use crate::server::{
    billing::types::base::BillingPlan,
    daemons::r#impl::base::DaemonMode,
    discovery::r#impl::types::{DiscoveryType, RunType},
    hosts::r#impl::{base::Host, virtualization::HostVirtualization},
    interfaces::r#impl::base::Interface,
    ip_addresses::r#impl::base::IPAddress,
    organizations::r#impl::base::OrgNotifications,
    ports::r#impl::base::Port,
    services::r#impl::{definitions::ServiceDefinition, virtualization::ServiceVirtualization},
    shared::{storage::filter::StorableFilter, types::entities::EntitySource},
    topology::types::{
        base::TopologyOptions,
        edges::{Edge, EdgeStyle},
        nodes::Node,
    },
    users::r#impl::{email_settings::EmailSettings, permissions::UserOrgPermissions},
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

/// The outcome of a lookup that expected to identify at most one row.
///
/// The third case is the point. `Option` has no way to say "the identifier you gave me does not
/// identify anything" as distinct from "nothing matched", so every lookup on a non-unique column
/// silently answered the first question with the second — see [`Storage::get_unique`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unique<T> {
    /// Exactly one row matched.
    One(T),
    /// Nothing matched.
    None,
    /// More than one row matched, so the filter does not identify a row.
    ///
    /// Deliberately carries no rows. Handing back candidates invites picking one, which is the
    /// behaviour this type exists to prevent.
    Multiple,
}

impl<T> Unique<T> {
    /// For a filter on a genuinely unique key — an id, an email, an API key.
    ///
    /// `Multiple` means a uniqueness assumption is broken, usually a constraint that was never
    /// added. That is worth an error rather than a silently chosen row: the caller asked for
    /// *the* user with this email, and there is no such thing.
    pub fn at_most_one(self) -> Result<Option<T>, anyhow::Error> {
        match self {
            Self::One(entity) => Ok(Some(entity)),
            Self::None => Ok(None),
            Self::Multiple => Err(anyhow::anyhow!(
                "expected at most one row, found several; a uniqueness assumption is broken"
            )),
        }
    }

    /// Apply `f` to the row, if there was exactly one.
    ///
    /// Lets a caller project a row onto the field it actually wanted — usually an id — without
    /// unwrapping to `Option` and losing the distinction between "nothing matched" and "the
    /// identifier does not identify".
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Unique<U> {
        match self {
            Self::One(entity) => Unique::One(f(entity)),
            Self::None => Unique::None,
            Self::Multiple => Unique::Multiple,
        }
    }

    /// The row, if the filter identified exactly one.
    ///
    /// For callers that treat "no match" and "ambiguous" alike. Prefer matching on the variants
    /// where the difference is worth reporting.
    pub fn found(self) -> Option<T> {
        match self {
            Self::One(entity) => Some(entity),
            Self::None | Self::Multiple => None,
        }
    }
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
    /// Fetch a row the filter is expected to identify uniquely.
    ///
    /// Replaced `get_one`, which was `fetch_optional` with no `ORDER BY` and no `LIMIT`: on a
    /// filter matching several rows it returned whichever one Postgres happened to emit first,
    /// indistinguishable from a genuine single match. That cost us a link drawn to an arbitrary
    /// port on switches repeating one MAC across every port (GH #668), and a lookup landing on an
    /// SCD2 snapshot copy instead of the live row.
    ///
    /// Returning [`Unique`] rather than `Option` is what makes the hazard unwriteable: a caller
    /// filtering on a non-unique column has to say what several matches mean, and a caller on a
    /// unique key says so out loud with [`Unique::at_most_one`].
    async fn get_unique(&self, filter: StorableFilter<T>) -> Result<Unique<T>, anyhow::Error>;
    /// Whether any row matches.
    ///
    /// For guards that ask "does this exist" rather than "which one is it". Deliberately not
    /// [`Self::get_unique`]: an existence check has nothing to say about uniqueness, and
    /// answering it with a lookup that treats several matches as an error turns a clear
    /// "delete the daemon first" into an internal error on exactly the data that needs the guard.
    async fn exists(&self, filter: StorableFilter<T>) -> Result<bool, anyhow::Error> {
        Ok(self.count(filter).await? > 0)
    }

    /// Count rows matching the filter (`SELECT COUNT(*)`), without fetching them.
    /// For internal count-only needs (dashboards, limit checks) — avoids the
    /// row fetch + tag hydration that `get_paginated`/`get_all` do.
    async fn count(&self, filter: StorableFilter<T>) -> Result<u64, anyhow::Error>;
    /// Count rows per distinct value of `group_sql` under the same filter,
    /// ignoring its limit/offset. `group_sql` is the ORDER BY expression the
    /// list query groups on, so a paginated list can report how big each group
    /// is in full rather than how much of it landed on the current page.
    /// Values come back rendered as text, `None` for a SQL NULL group.
    async fn count_by_group(
        &self,
        filter: StorableFilter<T>,
        group_sql: &str,
    ) -> Result<Vec<(Option<String>, u64)>, anyhow::Error>;
    async fn update(&self, entity: &mut T) -> Result<T, anyhow::Error>;
    async fn delete(&self, id: &Uuid) -> Result<(), anyhow::Error>;
    async fn create_many(&self, entities: &[T]) -> Result<Vec<T>, anyhow::Error>;
    async fn update_many(&self, entities: &[T]) -> Result<Vec<T>, anyhow::Error>;
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

    /// SQL boolean fragments that `StorableFilter::text_search` ORs together to
    /// implement free-text search over this entity. `{}` marks where the bound
    /// `%pattern%` parameter is substituted; the same parameter is reused by
    /// every fragment, so each may reference it once.
    ///
    /// Fragments are table-qualified and may reach into child tables via
    /// `EXISTS` — the same latitude `OrderField::join_sql` already takes.
    ///
    /// The default is empty, which makes `text_search` match *nothing* rather
    /// than everything: an entity that has not opted in cannot silently answer
    /// a search request with its full table.
    fn search_predicates() -> &'static [&'static str] {
        &[]
    }

    /// Whether this entity carries SCD2 columns (`valid_from` / `valid_to`).
    ///
    /// When `true`, reads filter to live rows (`valid_to IS NULL`) so closed historical
    /// copies don't leak: frontend-facing GET handlers drop them from list responses,
    /// `GET /{id}` of a closed row 404s, and [`ChildStorage`] excludes them when loading
    /// children for a parent.
    ///
    /// **Deliberately has no default.** It used to default to `false`, which meant an entity
    /// whose table had the columns but whose impl stayed silent was indistinguishable from one
    /// that genuinely had none. Three of them accumulated that way (`dependency_members`,
    /// `entity_tags`, `subnet_vlans`), unnoticed only because `ChildStorage` hard-coded the
    /// filter instead of consulting this. Answer it for every entity; the value must match
    /// what the table actually has.
    const HAS_SCD2: bool;

    /// Whether this entity instance is a live row (i.e. `valid_to IS NULL`).
    /// Default `true` for non-SCD2 entities; SCD2 types override to consult
    /// the `valid_to` field on `self`. Used by `get_by_id_handler` to 404
    /// closed historical copies.
    fn is_live_row(&self) -> bool {
        true
    }
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
    /// Default implementation delegates to `EntityDiscriminants::is_taggable`.
    fn is_taggable() -> bool {
        Self::entity_type().is_taggable()
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
#[derive(Clone, strum_macros::EnumDiscriminants)]
#[strum_discriminants(derive(strum_macros::EnumIter))]
pub enum SqlValue {
    Uuid(Uuid),
    OptionalUuid(Option<Uuid>),
    String(String),
    OptionalString(Option<String>),
    I32(i32),
    OptionalI32(Option<i32>),
    I64(i64),
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
    AttributeSource(AttributeSource),
    EntityDiscriminant(EntityDiscriminants),
    ServiceDefinition(Box<dyn ServiceDefinition>),
    OptionalServiceVirtualization(Option<ServiceVirtualization>),
    OptionalHostVirtualization(Option<HostVirtualization>),
    Ports(Vec<Port>),
    IPAddresses(Vec<IPAddress>),
    RunType(RunType),
    DiscoveryType(DiscoveryType),
    UserOrgPermissions(UserOrgPermissions),
    EmailSettings(EmailSettings),
    OptionBillingPlan(Option<BillingPlan>),
    OptionBillingPlanStatus(Option<SubscriptionStatus>),
    BillingOperation(BillingOperation),
    AuthenticatedEntity(AuthenticatedEntity),
    EdgeStyle(EdgeStyle),
    DaemonMode(DaemonMode),
    Nodes(std::collections::HashMap<TopologyView, Vec<Node>>),
    Edges(std::collections::HashMap<TopologyView, Vec<Edge>>),
    TopologyOptions(TopologyOptions),
    Hosts(Vec<Host>),
    Subnets(Vec<Subnet>),
    Services(Vec<Service>),
    Bindings(Vec<Binding>),
    Dependencies(Vec<Dependency>),
    OnboardingOperation(Vec<OnboardingOperationDiscriminants>),
    StringArray(Vec<String>),
    OptionalStringArray(Option<Vec<String>>),
    OptionalLldpChassisId(Option<LldpChassisId>),
    OptionalLldpPortId(Option<LldpPortId>),
    OptionalFdbMacs(Option<Vec<String>>),
    OptionVecU16(Option<Vec<u16>>),
    OptionVecUuid(Option<Vec<Uuid>>),
    ShareOptions(ShareOptions),
    EnabledViews(Option<Vec<TopologyView>>),
    CredentialType(CredentialType),
    MacAddress(MacAddress),
    OptionalMacAddress(Option<MacAddress>),
    MacAddressArray(Vec<MacAddress>),
    Interfaces(Vec<Interface>),
    Tags(Vec<Tag>),
    Vlans(Vec<Vlan>),
    OrgNotifications(OrgNotifications),
    OptionalUuidVec(Option<Vec<Uuid>>),
    IntegrationTargets(Vec<IntegrationTarget>),
}

// ============================================================================
// DB-backed enum catalog for backward-compat tests
// ============================================================================
//
// `SqlValue` is the complete typed catalog of everything the storage layer
// writes to the DB. By walking every `SqlValue` variant and contributing the
// variant names of each DB-backed Rust enum it transitively reaches, we build
// a baseline that lets us detect:
//
//   - removed/renamed variants that break an upgraded binary reading old rows
//     (forward-compat — Test A in tests.rs)
//   - added variants that break an old binary reading new rows during deploy
//     coexistence (backward-compat — Test B in tests.rs)
//
// Two compile-time gates ensure the catalog can't silently drift:
//   1. The `match` in `SqlValue::contribute_db_enum_variants` is exhaustive —
//      adding a new `SqlValue` variant fails the build.
//   2. Each non-primitive arm dispatches via the `DbEnumContributor` trait —
//      wrapping a type that doesn't implement the trait fails the build.

/// A type whose DB-persisted variant names (if any) should flow into the
/// backward-compat baseline fixture. Empty impls are legal — not every type
/// wrapped by `SqlValue` has DB-backed enums.
pub trait DbEnumContributor {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>);
}

/// Bare type name from `std::any::type_name`, stripping the module path.
/// Used as the fixture key so renames refactor automatically.
pub fn db_enum_key_for<T: ?Sized>() -> &'static str {
    std::any::type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or("?")
}

/// Empty impls for types that wrap no DB-backed enums (primitives, foreign
/// types, composite structs whose enum fields are covered elsewhere).
macro_rules! impl_db_enum_contributor_empty {
    ($($t:ty),* $(,)?) => {
        $(
            impl DbEnumContributor for $t {
                fn contribute(_: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {}
            }
        )*
    };
}

/// Populating impls for DB-backed enums. Uses `strum::VariantNames` to obtain
/// the list of variant identifiers at compile time — no instance construction
/// needed, so enums with non-`Default` payload fields work here too.
///
/// Gotcha: `strum::VariantNames` returns Rust identifiers. If a variant carries
/// `#[serde(rename = "...")]`, the DB-persisted tag differs from the Rust name;
/// the baseline will need manual extension to include the renamed form.
macro_rules! impl_db_enum_contributor_via_variant_names {
    ($($t:ty),* $(,)?) => {
        $(
            impl DbEnumContributor for $t {
                fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
                    let variants: Vec<String> = <$t as ::strum::VariantNames>::VARIANTS
                        .iter()
                        .map(|s| s.to_string())
                        .collect();
                    out.insert(db_enum_key_for::<$t>(), variants);
                }
            }
        )*
    };
}

// Primitives and foreign types wrapped in SqlValue — no DB-backed enums.
impl_db_enum_contributor_empty!(
    Uuid,
    String,
    i32,
    i64,
    u16,
    u8,
    bool,
    EmailAddress,
    DateTime<Utc>,
    IpCidr,
    IpAddr,
    MacAddress,
);

// Generic wrapper pass-through — inner type's contribution flows up.
impl<T: DbEnumContributor> DbEnumContributor for Option<T> {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
        T::contribute(out);
    }
}
impl<T: DbEnumContributor> DbEnumContributor for Vec<T> {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
        T::contribute(out);
    }
}

// Trait object: no enumerable variants. Dynamic dispatch through
/// `RunType` is the one composite whose nested enums are reachable nowhere else.
///
/// `RunType::Historical` boxes a whole `DiscoveryUpdatePayload`, and that payload's `warnings` are
/// coded — so the warning code and the three enums that fill its slots are all persisted JSONB
/// values that `strum::VariantNames` on `RunType` alone cannot see. This is the delegation the
/// note above the empty impls calls for: without it, adding a warning code would slip past both
/// coexistence gates while being written to the database.
impl DbEnumContributor for RunType {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
        let variants: Vec<String> = <RunType as ::strum::VariantNames>::VARIANTS
            .iter()
            .map(|s| s.to_string())
            .collect();
        out.insert(db_enum_key_for::<RunType>(), variants);

        DiscoveryWarningCode::contribute(out);
        SnmpWalkGroup::contribute(out);
        ClaimSource::contribute(out);
        MalformedNeighbourConsequence::contribute(out);
        CredentialQueryPayloadDiscriminants::contribute(out);
    }
}

/// `AttributeSource` is the second composite whose nested enum is reachable nowhere else.
///
/// Two of its variants carry a [`ClientProbe`] in the persisted JSON (`{"Probe":"Snmp"}`), so the
/// probe names are values written to the database that
/// `strum::VariantNames` on `AttributeSource` alone cannot see. Same delegation, same reason, as
/// `RunType` above: without it, adding a probe would slip past both coexistence gates while being
/// written to every `*_source` column.
impl DbEnumContributor for AttributeSource {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
        let variants: Vec<String> =
            <AttributeSourceDiscriminants as ::strum::VariantNames>::VARIANTS
                .iter()
                .map(|s| s.to_string())
                .collect();
        out.insert(db_enum_key_for::<AttributeSource>(), variants);

        ClientProbe::contribute(out);
    }
}

// ServiceDefinition covers service metadata (Docker, nginx, etc.), not
// DB-persisted discriminants — out of scope for this catalog.
impl DbEnumContributor for Box<dyn ServiceDefinition> {
    fn contribute(_: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {}
}

// Composite structs wrapped in SqlValue. Their enum fields (if any) are
// already reachable directly through other SqlValue variants, so these
// contribute nothing themselves. If a composite gains a nested enum that's
// NOT reachable elsewhere, replace the empty impl with one that delegates
// to the nested enum's `contribute`.
impl_db_enum_contributor_empty!(
    UserOrgPermissions,
    EmailSettings,
    TopologyOptions,
    ShareOptions,
    OrgNotifications,
    Port,
    IPAddress,
    Host,
    Subnet,
    Service,
    Binding,
    Dependency,
    Interface,
    Tag,
    Vlan,
    Node,
    Edge,
);

// DB-backed enums. Each gets a variant-names contribution. Requires
// `#[derive(strum::VariantNames)]` on the enum definition.
//
// Note: `SubscriptionStatus` is a foreign type from the `stripe_billing`
// crate — we can't add derives to it. Treated as empty below (Stripe SDK
// version bumps are explicit and coordinated with server deploys, so the
// coexistence-window risk is negligible in practice).
// The enums that ride inside a discovery session's coded warnings. Not reachable through any
// `SqlValue` variant of their own — they are nested inside `RunType::Historical` — so `RunType`'s
// contributor below delegates to them explicitly. Without that the coexistence gate would pass
// while the current binary was writing warning codes the previous release cannot read.
impl_db_enum_contributor_via_variant_names!(
    DiscoveryWarningCode,
    SnmpWalkGroup,
    ClaimSource,
    MalformedNeighbourConsequence,
    CredentialQueryPayloadDiscriminants,
);

impl_db_enum_contributor_via_variant_names!(
    EntitySource,
    ClientProbe,
    HostVirtualization,
    ServiceVirtualization,
    DiscoveryType,
    BillingPlan,
    EdgeStyle,
    DaemonMode,
    CredentialType,
    LldpChassisId,
    LldpPortId,
    TopologyView,
    crate::server::billing::types::base::CancelReason,
    crate::server::billing::types::base::SaveOffer,
    crate::server::billing::types::base::LimitType,
    crate::server::billing::types::base::LimitSource,
    AuthenticatedEntity,
    OnboardingOperationDiscriminants,
);

// IntegrationTarget is a tagged sum with struct variants (confuses strum::VariantNames on the
// enum itself). The persisted `scope` tag values are its `Target` discriminant's variant names,
// so source them from `Target::VARIANTS` rather than hand-typed strings.
impl DbEnumContributor for IntegrationTarget {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
        out.insert(
            db_enum_key_for::<IntegrationTarget>(),
            <Target as ::strum::VariantNames>::VARIANTS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        );
    }
}

// BillingOperation is a typed sum with payload-bearing variants. Its
// `Discriminants` enum carries the variant names; we contribute via that.
impl DbEnumContributor for BillingOperation {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
        // BillingOperation itself doesn't impl VariantNames (struct variants
        // confuse strum::VariantNames), but the Discriminants enum does not
        // either by default — list variant names explicitly to keep the
        // backward-compat baseline accurate.
        let variants = vec![
            "CheckoutStarted".to_string(),
            "CheckoutCompleted".to_string(),
            "TrialStarted".to_string(),
            "TrialWillEnd".to_string(),
            "TrialEnded".to_string(),
            "PlanChanged".to_string(),
            "SubscriptionCancelled".to_string(),
            "PaymentFailed".to_string(),
            "PaymentActionRequired".to_string(),
            "PaymentRecovered".to_string(),
            "FeatureLimitHit".to_string(),
            "Paused".to_string(),
            "Resumed".to_string(),
            "TrialExtended".to_string(),
            "CancellationInitiated".to_string(),
            "PaymentMethodAdded".to_string(),
            "PaymentMethodRemoved".to_string(),
        ];
        out.insert(db_enum_key_for::<Self>(), variants);
    }
}

// SubscriptionStatus: foreign type from stripe_billing. Empty impl.
impl DbEnumContributor for SubscriptionStatus {
    fn contribute(_: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {}
}

// EntityDiscriminants: auto-generated from Entity via EnumDiscriminants. The
// derive list includes VariantNames (see backend/src/server/shared/entities.rs).
impl DbEnumContributor for EntityDiscriminants {
    fn contribute(out: &mut std::collections::BTreeMap<&'static str, Vec<String>>) {
        let variants: Vec<String> = <Self as ::strum::VariantNames>::VARIANTS
            .iter()
            .map(|s| s.to_string())
            .collect();
        out.insert(db_enum_key_for::<Self>(), variants);
    }
}

impl SqlValue {
    /// For each `SqlValue` variant, contribute the DB-backed enum variant
    /// names reachable through its wrapped type. Exhaustive match on
    /// `SqlValueDiscriminants` forces every variant to be covered.
    fn dispatch_kind(
        kind: SqlValueDiscriminants,
        out: &mut std::collections::BTreeMap<&'static str, Vec<String>>,
    ) {
        use crate::server::lldp::{LldpChassisId, LldpPortId};
        use ShareOptions;
        use TopologyView;
        use Vlan;

        match kind {
            SqlValueDiscriminants::Uuid
            | SqlValueDiscriminants::OptionalUuid
            | SqlValueDiscriminants::UuidArray
            | SqlValueDiscriminants::OptionVecUuid
            | SqlValueDiscriminants::OptionalUuidVec => Uuid::contribute(out),
            SqlValueDiscriminants::String
            | SqlValueDiscriminants::OptionalString
            | SqlValueDiscriminants::StringArray
            | SqlValueDiscriminants::OptionalStringArray
            | SqlValueDiscriminants::OptionalFdbMacs => String::contribute(out),
            SqlValueDiscriminants::I32 | SqlValueDiscriminants::OptionalI32 => i32::contribute(out),
            SqlValueDiscriminants::I64 => i64::contribute(out),
            SqlValueDiscriminants::OptionalI64 => i64::contribute(out),
            SqlValueDiscriminants::U16 | SqlValueDiscriminants::OptionVecU16 => {
                u16::contribute(out)
            }
            SqlValueDiscriminants::Bool => bool::contribute(out),
            SqlValueDiscriminants::Email => EmailAddress::contribute(out),
            SqlValueDiscriminants::Timestamp | SqlValueDiscriminants::OptionTimestamp => {
                <DateTime<Utc>>::contribute(out)
            }
            SqlValueDiscriminants::IpCidr => IpCidr::contribute(out),
            SqlValueDiscriminants::IpAddr | SqlValueDiscriminants::OptionalIpAddr => {
                IpAddr::contribute(out)
            }
            SqlValueDiscriminants::MacAddress
            | SqlValueDiscriminants::OptionalMacAddress
            | SqlValueDiscriminants::MacAddressArray => MacAddress::contribute(out),
            SqlValueDiscriminants::EntitySource => EntitySource::contribute(out),
            SqlValueDiscriminants::AttributeSource => AttributeSource::contribute(out),
            SqlValueDiscriminants::EntityDiscriminant => EntityDiscriminants::contribute(out),
            SqlValueDiscriminants::ServiceDefinition => {
                <Box<dyn ServiceDefinition>>::contribute(out)
            }
            SqlValueDiscriminants::OptionalServiceVirtualization => {
                ServiceVirtualization::contribute(out)
            }
            SqlValueDiscriminants::OptionalHostVirtualization => {
                HostVirtualization::contribute(out)
            }
            SqlValueDiscriminants::Ports => Port::contribute(out),
            SqlValueDiscriminants::IPAddresses => IPAddress::contribute(out),
            SqlValueDiscriminants::RunType => RunType::contribute(out),
            SqlValueDiscriminants::DiscoveryType => DiscoveryType::contribute(out),
            SqlValueDiscriminants::UserOrgPermissions => UserOrgPermissions::contribute(out),
            SqlValueDiscriminants::EmailSettings => EmailSettings::contribute(out),
            SqlValueDiscriminants::OptionBillingPlan => BillingPlan::contribute(out),
            SqlValueDiscriminants::OptionBillingPlanStatus => SubscriptionStatus::contribute(out),
            SqlValueDiscriminants::BillingOperation => BillingOperation::contribute(out),
            SqlValueDiscriminants::AuthenticatedEntity => AuthenticatedEntity::contribute(out),
            SqlValueDiscriminants::EdgeStyle => EdgeStyle::contribute(out),
            SqlValueDiscriminants::DaemonMode => DaemonMode::contribute(out),
            SqlValueDiscriminants::Nodes => Node::contribute(out),
            SqlValueDiscriminants::Edges => Edge::contribute(out),
            SqlValueDiscriminants::TopologyOptions => TopologyOptions::contribute(out),
            SqlValueDiscriminants::Hosts => Host::contribute(out),
            SqlValueDiscriminants::Subnets => Subnet::contribute(out),
            SqlValueDiscriminants::Services => Service::contribute(out),
            SqlValueDiscriminants::Bindings => Binding::contribute(out),
            SqlValueDiscriminants::Dependencies => Dependency::contribute(out),
            SqlValueDiscriminants::OnboardingOperation => {
                OnboardingOperationDiscriminants::contribute(out)
            }
            SqlValueDiscriminants::OptionalLldpChassisId => LldpChassisId::contribute(out),
            SqlValueDiscriminants::OptionalLldpPortId => LldpPortId::contribute(out),
            SqlValueDiscriminants::ShareOptions => ShareOptions::contribute(out),
            SqlValueDiscriminants::EnabledViews => TopologyView::contribute(out),
            SqlValueDiscriminants::CredentialType => CredentialType::contribute(out),
            SqlValueDiscriminants::Interfaces => Interface::contribute(out),
            SqlValueDiscriminants::Tags => Tag::contribute(out),
            SqlValueDiscriminants::Vlans => Vlan::contribute(out),
            SqlValueDiscriminants::OrgNotifications => OrgNotifications::contribute(out),
            SqlValueDiscriminants::IntegrationTargets => IntegrationTarget::contribute(out),
        }
    }

    /// Produces the DB-backed enum baseline for the current binary: every
    /// DB-backed Rust enum reachable through any `SqlValue` variant, with its
    /// known variant names.
    pub fn collect_all_db_enum_variants() -> std::collections::BTreeMap<&'static str, Vec<String>> {
        use strum::IntoEnumIterator;
        let mut out = std::collections::BTreeMap::new();
        for kind in SqlValueDiscriminants::iter() {
            Self::dispatch_kind(kind, &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::Unique;

    /// The contract every unique-key caller relies on. `Multiple` means the uniqueness the
    /// lookup assumed is broken — a constraint that was never added, or a filter on the wrong
    /// column — and collapsing it to `None` would silently restore the class of defect this
    /// type replaced, across every site that spells `.at_most_one()?`.
    #[test]
    fn at_most_one_reports_an_error_rather_than_choosing() {
        assert_eq!(Unique::One(7).at_most_one().unwrap(), Some(7));
        assert_eq!(Unique::<i32>::None.at_most_one().unwrap(), None);
        assert!(
            Unique::<i32>::Multiple.at_most_one().is_err(),
            "several rows for a unique key must fail loudly, not pick one"
        );
    }

    /// `found` is for callers that act the same way on "nothing matched" and "the identifier
    /// does not identify" — it must never hand back a row it could not prove unique.
    #[test]
    fn found_yields_nothing_when_the_filter_did_not_identify_a_row() {
        assert_eq!(Unique::One(7).found(), Some(7));
        assert_eq!(Unique::<i32>::None.found(), None);
        assert_eq!(Unique::<i32>::Multiple.found(), None);
    }
}
