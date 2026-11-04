//! Generic typed event types.
//!
//! `Event<Op>` is parameterized over an `Operation` impl. Each operation type
//! carries per-domain `Scope` (identity dimensions: org_id / site_id / etc.),
//! `Flags` (cross-cutting hints like topology staleness), and `Filter`
//! (the shape of selection predicates a subscriber declares). How an event is
//! logged travels on the event itself, as an [`EventLog`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{borrow::Cow, collections::HashMap, fmt::Debug, hash::Hash, net::IpAddr};
use strum::IntoDiscriminant;
use uuid::Uuid;

use crate::daemon::discovery::types::base::DiscoveryTerminalReason;
use crate::server::{
    auth::middleware::auth::{ActorProperties, AuthenticatedEntity},
    daemons::r#impl::version::own_version,
    discovery::r#impl::types::DiscoveryType,
    shared::{
        entities::{Entity, EntityDiscriminants},
        events::{
            log::EventLog,
            types::{EntityOperation, EntityOperationDiscriminants, EventLogLevel, LabelColor},
        },
    },
};

// ===========================================================================
// Operation trait
// ===========================================================================

/// An operation enum for a typed event. Implementors are sum types with
/// `#[derive(EnumDiscriminants)]`. The discriminant enum is what filters key
/// on.
///
/// Each operation type carries:
/// - `Scope`: identity dimensions (org_id / site_id / entity / etc.)
/// - `Flags`: cross-cutting hints (topology staleness)
/// - `Filter`: the filter shape a `Subscriber<Self>` declares
///
/// All discriminant trait-bound repetition is consolidated here via an
/// associated-type bound so call sites don't need to repeat it.
pub trait Operation:
    IntoDiscriminant<
        Discriminant: Eq
                          + Hash
                          + Clone
                          + Debug
                          + Send
                          + Sync
                          + Serialize
                          + DeserializeOwned
                          + AsRef<str>
                          + 'static,
    > + Serialize
    + DeserializeOwned
    + Clone
    + Debug
    + Send
    + Sync
    + Sized
    + 'static
{
    type Scope: EventScope + Clone + Debug + Send + Sync + Serialize + DeserializeOwned + 'static;
    type Flags: Default + Clone + Debug + Send + Sync + Serialize + DeserializeOwned + 'static;
    type Filter: SubscriberFilter<Self>;

    fn log_level(&self) -> EventLogLevel;

    /// The event's snake_case name, e.g. `"subnet_created"`: the one name every consumer outside
    /// the bus (log line, PostHog) uses. Receives the `Scope` because some operations (notably
    /// entity events) carry the meaningful noun there rather than on the operation itself.
    fn event_name(&self, scope: &Self::Scope) -> Cow<'static, str>;

    /// Label prefixed to the event's log line. Defaults to [`Operation::event_name`].
    fn log_label(&self, scope: &Self::Scope) -> String {
        self.event_name(scope).into_owned()
    }

    /// Whom the event is about, for consumers that attribute events to a person.
    fn attribution(&self, _scope: &Self::Scope) -> Attribution {
        Attribution::Actor
    }

    /// The event-specific payload a consumer outside the server receives, under `metadata`.
    /// Defaults to the operation itself. Operations whose identity lives on the scope override
    /// it to pick the scope fields that are safe to send: a scope can carry the whole entity, a
    /// client address or a device's diagnostic.
    fn metadata<'a>(&'a self, _scope: &'a Self::Scope) -> impl Serialize + Send + 'a {
        self
    }

    /// Color for the label in the log line. Defaults to `Neutral`; operation
    /// types with create/update/delete semantics override this (see
    /// `EntityOperation`).
    fn log_color(&self) -> LabelColor {
        LabelColor::Neutral
    }

    /// How the logging subscriber writes this event: the whole event by default, with a readable
    /// line for what the JSON doesn't say plainly or leaves out (ids kept from PostHog). Entity
    /// events are decided by the entity instead, which sees both versions (see
    /// [`EntityLog`](crate::server::shared::events::log::EntityLog)).
    fn log(&self, _scope: &Self::Scope) -> EventLog {
        EventLog::default()
    }
}

// ===========================================================================
// Scope types
// ===========================================================================

/// The organization an event belongs to, as its scope names it. A site-scoped event names its
/// site; the consumer looks the site's organization up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeOrganization {
    Org(Uuid),
    Site(Uuid),
}

/// Implemented by every operation's `Scope`.
pub trait EventScope {
    /// `None` only where the scope has no organization at all, e.g. a failed login.
    fn organization(&self) -> Option<ScopeOrganization>;
}

/// Whom an event is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribution {
    /// The authenticated user, else the event's organization.
    Actor,
    /// A named user, e.g. an email's recipient.
    User(Uuid),
    /// The organization, whoever acted, e.g. an anonymous share view.
    Organization,
}

/// Identity scope for org-only events: `BillingOperation`, `OnboardingOperation`,
/// `AnalyticsOperation`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct OrgScope {
    pub organization_id: Uuid,
}

/// Identity scope for site-only events: `DiscoveryPhase`. Discovery sessions
/// are site-keyed; org is derivable via the site.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SiteScope {
    pub site_id: Uuid,
}

/// Identity scope for auth events. Both `user_id` and `organization_id` are
/// optional because failed-login events have neither.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AuthScope {
    pub user_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub ip_address: IpAddr,
    pub user_agent: Option<String>,
}

/// Identity scope for entity events. Entities are either org-scoped (User,
/// Invite, ApiKey, Organization) or site-scoped (Host, Subnet, Service,
/// Daemon, Tag, etc.) — never both.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityScope {
    Org {
        organization_id: Uuid,
        entity_id: Uuid,
        entity_type: Entity,
    },
    Site {
        site_id: Uuid,
        entity_id: Uuid,
        entity_type: Entity,
    },
}

impl EntityScope {
    /// Build a scope from the entity-event source fields: prefers
    /// `organization_id` when present (org-scoped entity), otherwise uses
    /// `site_id`. At least one must be `Some` — otherwise this returns
    /// `None`.
    pub fn from_ids(
        entity_id: Uuid,
        entity_type: Entity,
        site_id: Option<Uuid>,
        organization_id: Option<Uuid>,
    ) -> Option<Self> {
        if let Some(organization_id) = organization_id {
            Some(EntityScope::Org {
                organization_id,
                entity_id,
                entity_type,
            })
        } else {
            site_id.map(|site_id| EntityScope::Site {
                site_id,
                entity_id,
                entity_type,
            })
        }
    }

    pub fn entity_id(&self) -> Uuid {
        match self {
            EntityScope::Org { entity_id, .. } | EntityScope::Site { entity_id, .. } => *entity_id,
        }
    }

    pub fn entity_type(&self) -> &Entity {
        match self {
            EntityScope::Org { entity_type, .. } | EntityScope::Site { entity_type, .. } => {
                entity_type
            }
        }
    }

    pub fn entity_discriminant(&self) -> EntityDiscriminants {
        self.entity_type().discriminant()
    }
}

impl EventScope for OrgScope {
    fn organization(&self) -> Option<ScopeOrganization> {
        Some(ScopeOrganization::Org(self.organization_id))
    }
}

impl EventScope for SiteScope {
    fn organization(&self) -> Option<ScopeOrganization> {
        Some(ScopeOrganization::Site(self.site_id))
    }
}

impl EventScope for AuthScope {
    fn organization(&self) -> Option<ScopeOrganization> {
        self.organization_id.map(ScopeOrganization::Org)
    }
}

impl EventScope for EntityScope {
    fn organization(&self) -> Option<ScopeOrganization> {
        Some(match self {
            EntityScope::Org {
                organization_id, ..
            } => ScopeOrganization::Org(*organization_id),
            EntityScope::Site { site_id, .. } => ScopeOrganization::Site(*site_id),
        })
    }
}

impl EventScope for DiscoveryScope {
    fn organization(&self) -> Option<ScopeOrganization> {
        Some(ScopeOrganization::Site(self.site_id))
    }
}

/// Identity scope for discovery session events. Carries the session/daemon/
/// discovery-type identifiers, plus an `error_reason` populated for `Failed`
/// / `Cancelled` phases. `reason` says why a terminal phase was reached, so
/// subscribers that only see the event (metrics, analytics) can tell a reaped stall from a
/// user cancel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DiscoveryScope {
    pub site_id: Uuid,
    pub session_id: Uuid,
    pub daemon_id: Uuid,
    pub discovery_type: DiscoveryType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<DiscoveryTerminalReason>,
    /// When the server last heard about the run, set at terminal: for a stall, how long it was
    /// silent. Stays out of the PostHog metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_update_at: Option<DateTime<Utc>>,
    /// The phase a reaped run was in when it stalled. Stays out of the PostHog metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_phase: Option<crate::daemon::discovery::types::base::DiscoveryPhase>,
}

// ===========================================================================
// Flags types
// ===========================================================================

/// Cross-cutting hint flags for entity events. `trigger_stale` and `clear_stale`
/// control topology-rebuild gating.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntityEventFlags {
    pub trigger_stale: bool,
    pub clear_stale: bool,
}

/// Cross-cutting hints for non-entity events. None today; the type stays so each operation
/// declares its `Flags` the same way.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EventFlags {}

// ===========================================================================
// Generic typed event
// ===========================================================================

/// Generic typed event parameterized over an `Operation`. The `Operation`
/// trait already requires Serialize/DeserializeOwned on `Self`, `Scope`, and
/// `Flags`, so the serde-derive's auto-generated bounds are sufficient.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "Op: Operation")]
pub struct Event<Op: Operation> {
    pub id: Uuid,
    pub scope: Op::Scope,
    pub operation: Op,
    pub flags: Op::Flags,
    pub timestamp: DateTime<Utc>,
    pub authentication: AuthenticatedEntity,
    /// How the logging subscriber writes it. Never serialized: it isn't part of the event a
    /// consumer receives.
    #[serde(skip)]
    pub log: EventLog,
}

impl<Op: Operation> Event<Op> {
    pub fn new(scope: Op::Scope, operation: Op, authentication: AuthenticatedEntity) -> Self {
        let log = operation.log(&scope);
        Self {
            id: Uuid::new_v4(),
            scope,
            operation,
            flags: Op::Flags::default(),
            timestamp: crate::server::shared::clock::now(),
            authentication,
            log,
        }
    }

    pub fn with_flags(mut self, flags: Op::Flags) -> Self {
        self.flags = flags;
        self
    }

    pub fn with_log(mut self, log: EventLog) -> Self {
        self.log = log;
        self
    }

    pub fn discriminant(&self) -> <Op as IntoDiscriminant>::Discriminant {
        self.operation.discriminant()
    }

    pub fn log_label(&self) -> String {
        self.operation.log_label(&self.scope)
    }

    pub fn name(&self) -> Cow<'static, str> {
        self.operation.event_name(&self.scope)
    }

    pub fn attribution(&self) -> Attribution {
        self.operation.attribution(&self.scope)
    }

    /// The event as a consumer outside the server receives it: who acted, this server's version,
    /// and the operation's [`metadata`](Operation::metadata). `site_organization_id` is the
    /// caller's lookup of a [`ScopeOrganization::Site`]; the organization falls back to the
    /// authenticated entity's when the scope yields none.
    pub fn properties(
        &self,
        site_organization_id: Option<Uuid>,
    ) -> EventProperties<impl Serialize + Send + '_> {
        let mut actor = ActorProperties::from(&self.authentication);
        let scope_organization_id = match self.scope.organization() {
            Some(ScopeOrganization::Org(id)) => Some(id),
            Some(ScopeOrganization::Site(_)) => site_organization_id,
            None => None,
        };
        if let Some(id) = scope_organization_id {
            actor.organization_id = Some(id);
        }
        EventProperties {
            actor,
            server_version: own_version(),
            metadata: self.operation.metadata(&self.scope),
        }
    }
}

/// See [`Event::properties`]. `metadata` holds every event-specific field, so the top level
/// is the same for every event.
#[derive(Debug, Serialize)]
pub struct EventProperties<M> {
    #[serde(flatten)]
    pub actor: ActorProperties,
    pub server_version: semver::Version,
    pub metadata: M,
}

impl<M> EventProperties<M> {
    /// Extend the metadata, e.g. with a count a consumer aggregated over several events.
    pub fn map_metadata<N>(self, f: impl FnOnce(M) -> N) -> EventProperties<N> {
        EventProperties {
            actor: self.actor,
            server_version: self.server_version,
            metadata: f(self.metadata),
        }
    }
}

/// Render an event as JSON. The logging subscriber prefixes each line with a
/// `<label>: ` tag, so the JSON payload begins after the first `": "`;
/// downstream consumers (vector / loki) split on that before parsing.
impl<Op: Operation> std::fmt::Display for Event<Op> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string(self) {
            Ok(s) => f.write_str(&s),
            Err(e) => write!(f, r#"{{"error":"serialize event: {}"}}"#, e),
        }
    }
}

// ===========================================================================
// Filter trait + per-Op filter shapes
// ===========================================================================

/// A subscriber-declared predicate. Each `Operation` type has an associated
/// `Filter` type that implements this trait — most use the generic
/// `EventFilter<Op>`, but `EntityOperation` uses the richer `EntityEventFilter`.
pub trait SubscriberFilter<Op: Operation>: Default + Clone + Send + Sync + 'static {
    fn matches(&self, event: &Event<Op>) -> bool;
}

/// Default filter — used by Billing, Onboarding, Analytics, Auth, Discovery.
/// Restrict to a set of operation discriminants, or match all when `None`.
#[derive(Debug, Clone)]
pub struct EventFilter<Op: Operation> {
    pub operations: Option<Vec<<Op as IntoDiscriminant>::Discriminant>>,
}

impl<Op: Operation> Default for EventFilter<Op> {
    fn default() -> Self {
        Self::all()
    }
}

impl<Op: Operation> EventFilter<Op> {
    /// Match every event of this operation type.
    pub fn all() -> Self {
        Self { operations: None }
    }

    /// Match only the listed operation discriminants.
    pub fn ops(operations: Vec<<Op as IntoDiscriminant>::Discriminant>) -> Self {
        Self {
            operations: Some(operations),
        }
    }
}

impl<Op: Operation> SubscriberFilter<Op> for EventFilter<Op> {
    fn matches(&self, event: &Event<Op>) -> bool {
        match &self.operations {
            None => true,
            Some(ops) => ops.contains(&event.discriminant()),
        }
    }
}

/// Richer filter for `EntityOperation` — declares per-entity-type op gates so
/// the framework can dispatch only to subscribers that care.
///
/// `entity_discriminants`:
/// - `None` = match all entity events.
/// - `Some(map)`: each entry's presence means "match this entity type"; the
///   inner `Option<Vec<...>>` restricts to the listed ops (or all if `None`).
#[derive(Debug, Clone, Default)]
pub struct EntityEventFilter {
    pub entity_discriminants:
        Option<HashMap<EntityDiscriminants, Option<Vec<EntityOperationDiscriminants>>>>,
}

impl EntityEventFilter {
    /// Match every entity event.
    pub fn all() -> Self {
        Self {
            entity_discriminants: None,
        }
    }

    /// Restrict by entity type and (optionally) by operation per entity type.
    pub fn by_entity(
        map: HashMap<EntityDiscriminants, Option<Vec<EntityOperationDiscriminants>>>,
    ) -> Self {
        Self {
            entity_discriminants: Some(map),
        }
    }
}

impl SubscriberFilter<EntityOperation> for EntityEventFilter {
    fn matches(&self, event: &Event<EntityOperation>) -> bool {
        let Some(map) = &self.entity_discriminants else {
            return true;
        };
        let entity_disc = event.scope.entity_discriminant();
        let Some(allowed_ops) = map.get(&entity_disc) else {
            return false;
        };
        match allowed_ops {
            None => true,
            Some(ops) => ops.contains(&event.discriminant()),
        }
    }
}

// ===========================================================================
// Subscriber trait
// ===========================================================================

/// A subscriber for one operation type. Cross-event subscribers (PostHog,
/// Logging, Metrics, Email) implement this multiple times — once per
/// operation type they care about. Registration on the `EventBus` is handled
/// uniformly via `inventory::submit!(SubscriberRegistration::new::<Self, Op>())`
/// next to the impl block; see `shared/events/registry.rs`.
///
/// Identity: each (Service, Op) pair is named by the registry as
/// `<service_snake>:<op_snake>` — no per-impl `name()` method.
#[async_trait::async_trait]
pub trait Subscriber<Op: Operation>: Send + Sync {
    fn filter(&self) -> Op::Filter;
    /// Apply `events`. Report every failure as `Err`; never log and continue, or the failure
    /// reaches neither the bus retry nor `scanopy_event_subscriber_errors_total`.
    ///
    /// A plain `Err` tells the bus that calling `handle` again with the same events is safe, and
    /// it retries. Return [`NonRetryable`] instead when side effects already applied would repeat
    /// on a re-run (some emails of a batch sent, a non-idempotent create done); the bus counts it
    /// and does not retry.
    async fn handle(&self, events: Vec<Event<Op>>) -> anyhow::Result<()>;
    fn debounce_window_ms(&self) -> u64 {
        0
    }
}

pub use super::dispatch::{
    DispatchTracker, NonRetryable, SubscriberErrorOutcome, SubscriberErrorReason,
    SubscriberFailure, SubscriberFailureReporter, TypedChannel,
};
