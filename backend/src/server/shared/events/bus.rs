use std::{sync::Arc, time::Duration};

use crate::{
    daemon::discovery::types::{base::DiscoveryPhase, warnings::DiscoveryWarningCode},
    server::{
        dependencies::r#impl::member_events::DependencyMemberChange,
        digest::payload::DiscoveryDigestOperation,
        setup::types::SetupOperation,
        shared::events::{
            traits::{
                DispatchTracker, Event, Operation, Subscriber, SubscriberFailureReporter,
                TypedChannel,
            },
            types::{AnalyticsOperation, AuthOperation, BillingOperation, EntityOperation},
        },
        subnets::r#impl::correction_events::SubnetCorrection,
    },
};

pub struct EventBus {
    pub billing_channel: TypedChannel<BillingOperation>,
    pub setup_channel: TypedChannel<SetupOperation>,
    pub analytics_channel: TypedChannel<AnalyticsOperation>,
    pub auth_channel: TypedChannel<AuthOperation>,
    pub entity_channel: TypedChannel<EntityOperation>,
    pub discovery_channel: TypedChannel<DiscoveryPhase>,
    pub discovery_digest_channel: TypedChannel<DiscoveryDigestOperation>,
    /// One event per coded scan warning. Separate from `discovery_channel` because warnings arrive
    /// from two producers, one of which runs after the terminal phase event has already gone out.
    pub discovery_warning_channel: TypedChannel<DiscoveryWarningCode>,
    /// One event per subnet range a reading corrected. Its own channel because it fires outside any
    /// scan — a daemon's interfaced subnets arrive on status, not on a discovery submission — so it
    /// has neither a session nor a daemon to hang off the warning scope.
    pub subnet_correction_channel: TypedChannel<SubnetCorrection>,
    /// One event per dependency member whose binding or service changed under it.
    pub dependency_member_channel: TypedChannel<DependencyMemberChange>,
    /// Shared by every channel: what `flush` and `shutdown` wait on.
    tracker: Arc<DispatchTracker>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        let tracker = Arc::new(DispatchTracker::default());
        Self {
            tracker: tracker.clone(),
            billing_channel: TypedChannel::with_tracker(tracker.clone()),
            setup_channel: TypedChannel::with_tracker(tracker.clone()),
            analytics_channel: TypedChannel::with_tracker(tracker.clone()),
            auth_channel: TypedChannel::with_tracker(tracker.clone()),
            entity_channel: TypedChannel::with_tracker(tracker.clone()),
            discovery_channel: TypedChannel::with_tracker(tracker.clone()),
            discovery_digest_channel: TypedChannel::with_tracker(tracker.clone()),
            discovery_warning_channel: TypedChannel::with_tracker(tracker.clone()),
            subnet_correction_channel: TypedChannel::with_tracker(tracker.clone()),
            dependency_member_channel: TypedChannel::with_tracker(tracker.clone()),
        }
    }

    /// Register a typed subscriber. Op is inferred from the subscriber's
    /// `Subscriber<Op>` impl; the bus routes to the right channel via
    /// `BusChannel<Op>`. `name` is the auto-generated identifier from the
    /// `SubscriberRegistration` entry — used for diagnostic logging.
    pub async fn register<Op>(&self, subscriber: Arc<dyn Subscriber<Op>>, name: &'static str)
    where
        Op: Operation,
        Self: BusChannel<Op>,
    {
        self.channel().register(subscriber, name).await
    }

    /// Publish a typed event. Op is inferred from the event's type; the bus
    /// routes to the right channel via `BusChannel<Op>`. Returns once the
    /// event is queued; subscribers handle it in the background.
    pub async fn publish<Op>(&self, event: Event<Op>)
    where
        Op: Operation,
        Self: BusChannel<Op>,
    {
        self.channel().publish(event).await
    }

    /// Send each subscriber failure on every channel to `reporter` as well as the log and the
    /// counter.
    pub fn set_failure_reporter(&self, reporter: Arc<dyn SubscriberFailureReporter>) {
        self.tracker.set_failure_reporter(reporter)
    }

    /// Wait until every published event has been handled by every subscriber, including events
    /// subscribers publish along the way.
    pub async fn flush(&self) {
        self.tracker.flush().await
    }

    /// Drain queued subscriber work before the server exits, waiting at most `budget`. Anything
    /// still queued after that is logged and dropped.
    pub async fn shutdown(&self, budget: Duration) {
        if tokio::time::timeout(budget, self.flush()).await.is_err() {
            tracing::error!(
                undelivered = self.tracker.in_flight(),
                budget_secs = budget.as_secs(),
                "Event subscribers did not drain before shutdown; dropping the rest"
            );
        }
    }
}

/// Maps an `Operation` type to its corresponding channel on the bus. One impl
/// per Op type — the only place the per-op-type plumbing knows about specific
/// channel fields.
pub trait BusChannel<Op: Operation> {
    fn channel(&self) -> &TypedChannel<Op>;
}

impl BusChannel<BillingOperation> for EventBus {
    fn channel(&self) -> &TypedChannel<BillingOperation> {
        &self.billing_channel
    }
}

impl BusChannel<SetupOperation> for EventBus {
    fn channel(&self) -> &TypedChannel<SetupOperation> {
        &self.setup_channel
    }
}

impl BusChannel<AnalyticsOperation> for EventBus {
    fn channel(&self) -> &TypedChannel<AnalyticsOperation> {
        &self.analytics_channel
    }
}

impl BusChannel<AuthOperation> for EventBus {
    fn channel(&self) -> &TypedChannel<AuthOperation> {
        &self.auth_channel
    }
}

impl BusChannel<EntityOperation> for EventBus {
    fn channel(&self) -> &TypedChannel<EntityOperation> {
        &self.entity_channel
    }
}

impl BusChannel<DiscoveryPhase> for EventBus {
    fn channel(&self) -> &TypedChannel<DiscoveryPhase> {
        &self.discovery_channel
    }
}

impl BusChannel<DiscoveryDigestOperation> for EventBus {
    fn channel(&self) -> &TypedChannel<DiscoveryDigestOperation> {
        &self.discovery_digest_channel
    }
}

impl BusChannel<DiscoveryWarningCode> for EventBus {
    fn channel(&self) -> &TypedChannel<DiscoveryWarningCode> {
        &self.discovery_warning_channel
    }
}

impl BusChannel<SubnetCorrection> for EventBus {
    fn channel(&self) -> &TypedChannel<SubnetCorrection> {
        &self.subnet_correction_channel
    }
}

impl BusChannel<DependencyMemberChange> for EventBus {
    fn channel(&self) -> &TypedChannel<DependencyMemberChange> {
        &self.dependency_member_channel
    }
}
