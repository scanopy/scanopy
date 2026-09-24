//! Generic typed event types.
//!
//! `Event<Op>` is parameterized over an `Operation` impl. Each operation type
//! carries per-domain `Scope` (identity dimensions: org_id / network_id / etc.),
//! `Flags` (cross-cutting emission hints like `suppress_logs`), and `Filter`
//! (the shape of selection predicates a subscriber declares).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{collections::HashMap, fmt::Debug, hash::Hash, net::IpAddr};
use std::{sync::Arc, time::Duration};
use strum::IntoDiscriminant;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

use crate::daemon::discovery::types::base::{DiscoveryPhase, DiscoveryTerminalReason};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    discovery::r#impl::types::DiscoveryType,
    shared::{
        entities::{Entity, EntityDiscriminants},
        events::types::{
            AnalyticsOperation, AnalyticsOperationDiscriminants, AuthOperation,
            AuthOperationDiscriminants, BillingOperation, BillingOperationDiscriminants,
            EntityOperation, EntityOperationDiscriminants, EventLogLevel, LabelColor,
            OnboardingOperation, OnboardingOperationDiscriminants,
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
/// - `Scope`: identity dimensions (org_id / network_id / entity / etc.)
/// - `Flags`: cross-cutting emission hints (`suppress_logs`, etc.)
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
    type Scope: Clone + Debug + Send + Sync + Serialize + DeserializeOwned + 'static;
    type Flags: Default + Clone + Debug + Send + Sync + Serialize + DeserializeOwned + 'static;
    type Filter: SubscriberFilter<Self>;

    fn log_level(&self) -> EventLogLevel;

    /// Human-scannable label prefixed to the event's log line, e.g.
    /// `"Created"`. Receives the `Scope` because some operations (notably
    /// entity events) carry the meaningful noun there rather than on the
    /// operation itself. Default is the operation discriminant's name.
    fn log_label(&self, _scope: &Self::Scope) -> String {
        self.discriminant().as_ref().to_string()
    }

    /// Color for the label in the log line. Defaults to `Neutral`; operation
    /// types with create/update/delete semantics override this (see
    /// `EntityOperation`).
    fn log_color(&self) -> LabelColor {
        LabelColor::Neutral
    }
}

// ===========================================================================
// Scope types
// ===========================================================================

/// Identity scope for org-only events: `BillingOperation`, `OnboardingOperation`,
/// `AnalyticsOperation`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct OrgScope {
    pub organization_id: Uuid,
}

/// Identity scope for network-only events: `DiscoveryPhase`. Discovery sessions
/// are network-keyed; org is derivable via the network.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NetworkScope {
    pub network_id: Uuid,
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
/// Invite, ApiKey, Organization) or network-scoped (Host, Subnet, Service,
/// Daemon, Tag, etc.) — never both.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityScope {
    Org {
        organization_id: Uuid,
        entity_id: Uuid,
        entity_type: Entity,
    },
    Network {
        network_id: Uuid,
        entity_id: Uuid,
        entity_type: Entity,
    },
}

impl EntityScope {
    /// Build a scope from the entity-event source fields: prefers
    /// `organization_id` when present (org-scoped entity), otherwise uses
    /// `network_id`. At least one must be `Some` — otherwise this returns
    /// `None`.
    pub fn from_ids(
        entity_id: Uuid,
        entity_type: Entity,
        network_id: Option<Uuid>,
        organization_id: Option<Uuid>,
    ) -> Option<Self> {
        if let Some(organization_id) = organization_id {
            Some(EntityScope::Org {
                organization_id,
                entity_id,
                entity_type,
            })
        } else {
            network_id.map(|network_id| EntityScope::Network {
                network_id,
                entity_id,
                entity_type,
            })
        }
    }

    pub fn entity_id(&self) -> Uuid {
        match self {
            EntityScope::Org { entity_id, .. } | EntityScope::Network { entity_id, .. } => {
                *entity_id
            }
        }
    }

    pub fn entity_type(&self) -> &Entity {
        match self {
            EntityScope::Org { entity_type, .. } | EntityScope::Network { entity_type, .. } => {
                entity_type
            }
        }
    }

    pub fn entity_discriminant(&self) -> EntityDiscriminants {
        self.entity_type().discriminant()
    }

    pub fn organization_id(&self) -> Option<Uuid> {
        match self {
            EntityScope::Org {
                organization_id, ..
            } => Some(*organization_id),
            EntityScope::Network { .. } => None,
        }
    }

    pub fn network_id(&self) -> Option<Uuid> {
        match self {
            EntityScope::Org { .. } => None,
            EntityScope::Network { network_id, .. } => Some(*network_id),
        }
    }
}

/// Identity scope for discovery session events. Carries the session/daemon/
/// discovery-type identifiers, plus an `error_reason` populated for `Failed`
/// / `Cancelled` phases. `reason` says why a terminal phase was reached, so
/// subscribers that only see the event (metrics, analytics) can tell a reaped stall from a
/// user cancel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DiscoveryScope {
    pub network_id: Uuid,
    pub session_id: Uuid,
    pub daemon_id: Uuid,
    pub discovery_type: DiscoveryType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<DiscoveryTerminalReason>,
}

// ===========================================================================
// Flags types
// ===========================================================================

/// Cross-cutting hint flags for entity events. `trigger_stale` and `clear_stale`
/// control topology-rebuild gating; `suppress_logs` keeps noisy emissions out
/// of logs and analytics.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntityEventFlags {
    pub suppress_logs: bool,
    pub trigger_stale: bool,
    pub clear_stale: bool,
}

/// Cross-cutting hints for non-entity events. Currently only `suppress_logs`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EventFlags {
    pub suppress_logs: bool,
}

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
}

impl<Op: Operation> Event<Op> {
    pub fn new(scope: Op::Scope, operation: Op, authentication: AuthenticatedEntity) -> Self {
        Self {
            id: Uuid::new_v4(),
            scope,
            operation,
            flags: Op::Flags::default(),
            timestamp: Utc::now(),
            authentication,
        }
    }

    pub fn with_flags(mut self, flags: Op::Flags) -> Self {
        self.flags = flags;
        self
    }

    pub fn discriminant(&self) -> <Op as IntoDiscriminant>::Discriminant {
        self.operation.discriminant()
    }

    pub fn log_label(&self) -> String {
        self.operation.log_label(&self.scope)
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
// Operation impls — log_level lives on the trait, not as inherent methods
// ===========================================================================

impl Operation for BillingOperation {
    type Scope = OrgScope;
    type Flags = EventFlags;
    type Filter = EventFilter<BillingOperation>;
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Info
    }
}

impl Operation for OnboardingOperation {
    type Scope = OrgScope;
    type Flags = EventFlags;
    type Filter = EventFilter<OnboardingOperation>;
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Info
    }
}

impl Operation for AnalyticsOperation {
    type Scope = OrgScope;
    type Flags = EventFlags;
    type Filter = EventFilter<AnalyticsOperation>;
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Debug
    }
}

impl Operation for AuthOperation {
    type Scope = AuthScope;
    type Flags = EventFlags;
    type Filter = EventFilter<AuthOperation>;
    fn log_level(&self) -> EventLogLevel {
        match self {
            AuthOperation::LoginFailed { .. } | AuthOperation::ApiKeyAuthFailed { .. } => {
                EventLogLevel::Warn
            }
            _ => EventLogLevel::Info,
        }
    }
}

impl Operation for EntityOperation {
    type Scope = EntityScope;
    type Flags = EntityEventFlags;
    type Filter = EntityEventFilter;
    fn log_level(&self) -> EventLogLevel {
        EventLogLevel::Info
    }

    /// Entity events carry the noun in the scope, so compose it with the
    /// operation, e.g. `"Subnet Created"`.
    fn log_label(&self, scope: &Self::Scope) -> String {
        format!(
            "{} {}",
            scope.entity_discriminant().as_ref(),
            self.discriminant().as_ref()
        )
    }

    fn log_color(&self) -> LabelColor {
        match self {
            EntityOperation::Created => LabelColor::Green,
            EntityOperation::Updated => LabelColor::Blue,
            EntityOperation::Deleted => LabelColor::Red,
            EntityOperation::Get | EntityOperation::GetAll => LabelColor::Neutral,
        }
    }
}

impl Operation for DiscoveryPhase {
    type Scope = DiscoveryScope;
    type Flags = EventFlags;
    type Filter = EventFilter<DiscoveryPhase>;
    fn log_level(&self) -> EventLogLevel {
        match self {
            DiscoveryPhase::Failed => EventLogLevel::Warn,
            _ => EventLogLevel::Info,
        }
    }
}

// Convenience aliases for the discriminant types so consumers don't have to
// write `<BillingOperation as IntoDiscriminant>::Discriminant`.
pub type BillingDiscriminant = BillingOperationDiscriminants;
pub type OnboardingDiscriminant = OnboardingOperationDiscriminants;
pub type AnalyticsDiscriminant = AnalyticsOperationDiscriminants;
pub type AuthDiscriminant = AuthOperationDiscriminants;
pub type EntityDiscriminant = EntityOperationDiscriminants;

// ===========================================================================
// Subscriber trait
// ===========================================================================

/// A subscriber for one operation type. Cross-event subscribers (PostHog,
/// Logging, Metrics, Email, Brevo) implement this multiple times — once per
/// operation type they care about. Registration on the `EventBus` is handled
/// uniformly via `inventory::submit!(SubscriberRegistration::new::<Self, Op>())`
/// next to the impl block; see `shared/events/registry.rs`.
///
/// Identity: each (Service, Op) pair is named by the registry as
/// `<service_snake>:<op_snake>` — no per-impl `name()` method.
#[async_trait::async_trait]
pub trait Subscriber<Op: Operation>: Send + Sync {
    fn filter(&self) -> Op::Filter;
    async fn handle(&self, events: Vec<Event<Op>>) -> anyhow::Result<()>;
    fn debounce_window_ms(&self) -> u64 {
        0
    }
}

// ===========================================================================
// Typed channel — per-operation-type pub/sub plumbing
// ===========================================================================

struct TypedSubscriberState<Op: Operation> {
    subscriber: Arc<dyn Subscriber<Op>>,
    name: &'static str,
    pending: Arc<RwLock<Vec<Event<Op>>>>,
}

impl<Op: Operation> TypedSubscriberState<Op> {
    fn new(subscriber: Arc<dyn Subscriber<Op>>, name: &'static str) -> Self {
        let debounce_ms = subscriber.debounce_window_ms();
        let pending = Arc::new(RwLock::new(Vec::<Event<Op>>::new()));

        if debounce_ms > 0 {
            let pending_clone = pending.clone();
            let subscriber_clone = subscriber.clone();
            let window = Duration::from_millis(debounce_ms);
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(window);
                loop {
                    interval.tick().await;
                    let drained: Vec<Event<Op>> = {
                        let mut p = pending_clone.write().await;
                        if p.is_empty() {
                            continue;
                        }
                        p.drain(..).collect()
                    };
                    if let Err(e) = subscriber_clone.handle(drained).await {
                        tracing::error!(
                            subscriber = name,
                            error = %e,
                            "Typed subscriber failed to handle batched events",
                        );
                    }
                }
            });
        }

        Self {
            subscriber,
            name,
            pending,
        }
    }

    async fn add_event(&self, event: Event<Op>) {
        if self.subscriber.debounce_window_ms() == 0 {
            if let Err(e) = self.subscriber.handle(vec![event]).await {
                tracing::error!(
                    subscriber = self.name,
                    error = %e,
                    "Typed subscriber failed to handle event",
                );
            }
        } else {
            self.pending.write().await.push(event);
        }
    }
}

/// One broadcast channel + subscriber list per operation type. Owned by
/// `EventBus`. `publish` fans out to all subscribers whose `filter().matches`
/// returns true.
pub struct TypedChannel<Op: Operation> {
    sender: broadcast::Sender<Event<Op>>,
    subscribers: Arc<RwLock<Vec<TypedSubscriberState<Op>>>>,
}

impl<Op: Operation> Default for TypedChannel<Op> {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            sender,
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl<Op: Operation> TypedChannel<Op> {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(&self, subscriber: Arc<dyn Subscriber<Op>>, name: &'static str) {
        let debounce_ms = subscriber.debounce_window_ms();
        let state = TypedSubscriberState::new(subscriber, name);
        let mut subs = self.subscribers.write().await;
        subs.push(state);
        tracing::debug!(
            subscriber = name,
            debounce_ms = debounce_ms,
            "Registered typed subscriber",
        );
    }

    pub async fn publish(&self, event: Event<Op>) -> anyhow::Result<()> {
        let _ = self.sender.send(event.clone());
        let subs = self.subscribers.read().await;
        for state in subs.iter() {
            if state.subscriber.filter().matches(&event) {
                state.add_event(event.clone()).await;
            }
        }
        Ok(())
    }

    pub fn subscribe_channel(&self) -> broadcast::Receiver<Event<Op>> {
        self.sender.subscribe()
    }
}
