//! Generic typed event types.
//!
//! `Event<Op>` is parameterized over an `Operation` impl. Each operation type
//! carries per-domain `Scope` (identity dimensions: org_id / site_id / etc.),
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
/// - `Scope`: identity dimensions (org_id / site_id / entity / etc.)
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

    pub fn organization_id(&self) -> Option<Uuid> {
        match self {
            EntityScope::Org {
                organization_id, ..
            } => Some(*organization_id),
            EntityScope::Site { .. } => None,
        }
    }

    pub fn site_id(&self) -> Option<Uuid> {
        match self {
            EntityScope::Org { .. } => None,
            EntityScope::Site { site_id, .. } => Some(*site_id),
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
    pub site_id: Uuid,
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

/// A subscriber failure that a re-run would make worse: side effects for part of the work have
/// already been applied, and running `handle` again would repeat them. `failed` is how many
/// events (or sends) failed, so the error counter reflects the size of the loss.
#[derive(Debug, thiserror::Error)]
#[error("{failed} failed: {source}")]
pub struct NonRetryable {
    pub failed: usize,
    #[source]
    pub source: anyhow::Error,
}

impl NonRetryable {
    /// `None` when `failures` is empty; otherwise the count and the first error.
    pub fn from_failures(failures: Vec<anyhow::Error>) -> Option<Self> {
        let failed = failures.len();
        failures
            .into_iter()
            .next()
            .map(|source| Self { failed, source })
    }
}

/// First wait between attempts of a failing subscriber (doubling after). Short, because the
/// inline path runs inside HTTP requests, Stripe webhooks among them.
const SUBSCRIBER_RETRY_MIN_DELAY: Duration = Duration::from_millis(200);
/// Retries after the first attempt.
const SUBSCRIBER_RETRY_TIMES: usize = 2;

/// How a delivery that hit an error ended. The `outcome` label of
/// `scanopy_event_subscriber_errors_total`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum SubscriberErrorOutcome {
    /// A later attempt succeeded.
    Recovered,
    /// Every attempt failed.
    Exhausted,
    /// Not retried: the subscriber said a re-run is unsafe, or the error can't clear in time.
    NotRetried,
}

/// Errors a retry can't fix within the budget: a [`NonRetryable`] report, a validation failure,
/// or an org lock that already waited its full timeout.
fn not_retryable(e: &anyhow::Error) -> bool {
    e.chain().any(|c| {
        c.is::<NonRetryable>()
            || c.is::<crate::server::shared::types::api::ValidationError>()
            || matches!(
                c.downcast_ref::<crate::server::shared::storage::lock::LockError>(),
                Some(crate::server::shared::storage::lock::LockError::Timeout(..))
            )
    })
}

/// Run `subscriber.handle`, retrying this subscriber alone (never the others on the bus, whose
/// side effects already ran) while the error is retryable and attempts remain. Records one
/// `scanopy_event_subscriber_errors_total` increment per delivery that hit an error.
async fn handle_with_retry<Op: Operation>(
    subscriber: &dyn Subscriber<Op>,
    name: &'static str,
    events: Vec<Event<Op>>,
) {
    use crate::server::metrics::subscriber::record_subscriber_error;
    use backon::{ExponentialBuilder, Retryable};

    let mut retries = 0usize;
    let result = (|| subscriber.handle(events.clone()))
        .retry(
            ExponentialBuilder::default()
                .with_min_delay(SUBSCRIBER_RETRY_MIN_DELAY)
                .with_max_times(SUBSCRIBER_RETRY_TIMES),
        )
        .when(|e| !not_retryable(e))
        .notify(|e, delay| {
            retries += 1;
            tracing::debug!(
                subscriber = name,
                retry = retries,
                retry_in_ms = delay.as_millis() as u64,
                error = format!("{e:#}"),
                "Typed subscriber failed; retrying"
            );
        })
        .await;
    let attempts = retries + 1;

    match result {
        Ok(()) if retries > 0 => {
            record_subscriber_error(name, SubscriberErrorOutcome::Recovered, 1);
            tracing::warn!(
                subscriber = name,
                attempts,
                "Typed subscriber recovered after retry"
            );
        }
        Ok(()) => {}
        Err(e) if not_retryable(&e) => {
            let failed = e
                .chain()
                .find_map(|c| c.downcast_ref::<NonRetryable>())
                .map_or(1, |n| n.failed);
            record_subscriber_error(name, SubscriberErrorOutcome::NotRetried, failed as u64);
            tracing::error!(
                subscriber = name,
                error = format!("{e:#}"),
                "Typed subscriber failed; not retried"
            );
        }
        Err(e) => {
            record_subscriber_error(name, SubscriberErrorOutcome::Exhausted, 1);
            tracing::error!(
                subscriber = name,
                attempts,
                error = format!("{e:#}"),
                "Typed subscriber failed after retries"
            );
        }
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
                    handle_with_retry(subscriber_clone.as_ref(), name, drained).await;
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
            handle_with_retry(self.subscriber.as_ref(), self.name, vec![event]).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::events::types::OnboardingOperation;

    use crate::server::shared::storage::lock::{LockError, LockKey};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Fails its first `failures` calls with the error `make_error` builds, then succeeds.
    struct ScriptedSubscriber {
        failures: usize,
        make_error: fn() -> anyhow::Error,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Subscriber<OnboardingOperation> for ScriptedSubscriber {
        fn filter(&self) -> EventFilter<OnboardingOperation> {
            EventFilter::all()
        }
        async fn handle(&self, _: Vec<Event<OnboardingOperation>>) -> anyhow::Result<()> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call < self.failures {
                return Err((self.make_error)());
            }
            Ok(())
        }
    }

    /// Publish one event to a channel holding only `subscriber`; return its call count and the
    /// rendered `scanopy_event_subscriber_errors_total` lines.
    async fn deliver(failures: usize, make_error: fn() -> anyhow::Error) -> (usize, String) {
        let recorder = crate::server::metrics::prometheus_builder().build_recorder();
        let handle = recorder.handle();
        let _guard = metrics::set_default_local_recorder(&recorder);

        let calls = Arc::new(AtomicUsize::new(0));
        let channel = TypedChannel::<OnboardingOperation>::new();
        channel
            .register(
                Arc::new(ScriptedSubscriber {
                    failures,
                    make_error,
                    calls: calls.clone(),
                }),
                "scripted_subscriber:onboarding_operation",
            )
            .await;
        channel
            .publish(Event::new(
                OrgScope {
                    organization_id: Uuid::new_v4(),
                },
                OnboardingOperation::OnboardingModalCompleted,
                AuthenticatedEntity::System,
            ))
            .await
            .expect("publish carries on past a failing subscriber");

        let rendered = handle
            .render()
            .lines()
            .filter(|l| l.starts_with("scanopy_event_subscriber_errors_total{"))
            .collect::<Vec<_>>()
            .join("\n");
        (calls.load(Ordering::SeqCst), rendered)
    }

    fn series(outcome: &str, value: u64) -> String {
        format!(
            r#"scanopy_event_subscriber_errors_total{{subscriber="scripted_subscriber",event_type="onboarding_operation",outcome="{outcome}"}} {value}"#
        )
    }

    fn transient() -> anyhow::Error {
        anyhow::anyhow!("connection reset")
    }

    #[tokio::test(flavor = "current_thread")]
    async fn transient_failure_is_retried_and_counted_as_recovered() {
        let (calls, rendered) = deliver(1, transient).await;
        assert_eq!(calls, 2);
        assert_eq!(rendered, series("recovered", 1));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn persistent_failure_stops_after_three_attempts() {
        let (calls, rendered) = deliver(usize::MAX, transient).await;
        assert_eq!(calls, 3);
        assert_eq!(rendered, series("exhausted", 1));
    }

    /// The lock already waited its full timeout; retrying would hold the request for another 30s.
    #[tokio::test(flavor = "current_thread")]
    async fn lock_timeout_is_not_retried() {
        let (calls, rendered) = deliver(usize::MAX, || {
            anyhow::Error::from(LockError::Timeout(
                Duration::from_secs(30),
                LockKey::Organization(Uuid::nil()),
            ))
            .context("org mirror write")
        })
        .await;
        assert_eq!(calls, 1);
        assert_eq!(rendered, series("not_retried", 1));
    }

    /// A subscriber that already applied part of its work reports the failures without a retry,
    /// and the counter carries how many failed.
    #[tokio::test(flavor = "current_thread")]
    async fn non_retryable_report_counts_each_failure() {
        let (calls, rendered) = deliver(usize::MAX, || {
            NonRetryable::from_failures(vec![transient(), transient(), transient()])
                .expect("three failures")
                .into()
        })
        .await;
        assert_eq!(calls, 1);
        assert_eq!(rendered, series("not_retried", 3));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn successful_delivery_records_nothing() {
        let (calls, rendered) = deliver(0, transient).await;
        assert_eq!(calls, 1);
        assert!(rendered.is_empty(), "{rendered}");
    }
}
