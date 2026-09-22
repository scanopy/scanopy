//! Metrics subscriber for every operation type.
//!
//! Increments a Prometheus counter per event. Entity events go to
//! `scanopy_entity_events_total{entity_type, operation}`; non-entity events
//! go to `scanopy_events_total{category, operation}`. Two metrics, two
//! cardinalities — entity dimensions stay separate from event categories.
//!
//! Discovery scan warnings get a third, `scanopy_discovery_warnings_total{code, integration}`,
//! because neither label fits the two above: the question an operator asks of a warning is which
//! failure mode and whose integration, not which entity changed.
//!
//! How sessions end gets `scanopy_discovery_terminal_total{phase, reason}`. The session duration
//! histogram and the active-sessions gauge live with the discovery service, which holds the
//! timestamps and the live session map.

use anyhow::Error;
use async_trait::async_trait;
use strum::IntoDiscriminant;

use crate::{
    daemon::discovery::types::{
        base::{DiscoveryPhase, DiscoveryTerminalReason},
        warnings::DiscoveryWarningCode,
    },
    server::{
        metrics::service::MetricsService,
        shared::types::metadata::HasId,
        shared::events::{
            registry::SubscriberRegistration,
            traits::{EntityEventFilter, Event, EventFilter, Subscriber},
            types::{
                AnalyticsOperation, AuthOperation, BillingOperation, EntityOperation,
                OnboardingOperation,
            },
        },
    },
};

/// Per-entity-type counter (Host/Service/Subnet/etc. × Created/Updated/Deleted/...).
fn record_entity(entity_type: &str, operation: impl std::fmt::Display) {
    metrics::counter!(
        "scanopy_entity_events_total",
        "entity_type" => entity_type.to_string(),
        "operation" => operation.to_string(),
    )
    .increment(1);
}

/// Per-category counter for non-entity events (billing/onboarding/analytics/...).
/// Kept distinct from entity events so per-host metrics aren't muddled with
/// per-login metrics.
fn record_event(category: &str, operation: impl std::fmt::Display) {
    metrics::counter!(
        "scanopy_events_total",
        "category" => category.to_string(),
        "operation" => operation.to_string(),
    )
    .increment(1);
}

/// Per-code, per-integration counter for discovery scan warnings.
///
/// **One increment per warning, and a warning is one occurrence** — a failure mode affecting twelve
/// devices increments by twelve, because the counter measures how much of the estate is affected
/// and that is what an operator sizing the problem is asking. "How many scans saw this at all" is a
/// different question, answered by the distinct-session count in the analytics events rather than
/// by collapsing this one.
///
/// Both labels are bounded discriminants: ~42 codes against 8 integrations plus `none`. Nothing an
/// occurrence carries — no address, no host id, no library diagnostic — reaches a label.
fn record_discovery_warning(code: DiscoveryWarningCode, integration: Option<impl ToString>) {
    metrics::counter!(
        "scanopy_discovery_warnings_total",
        "code" => code.to_string(),
        "integration" => integration.map(|i| i.to_string()).unwrap_or_else(|| "none".to_string()),
    )
    .increment(1);
}

/// How discovery sessions end, by phase and terminal reason.
///
/// A separate series rather than a `reason` label on `scanopy_events_total`, which would put the
/// label on every category. `reason` is `none` for a terminal event with no reason, which only a
/// server older than the reason field publishes. Both labels are bounded: three terminal phases
/// against eight reasons.
fn record_discovery_terminal(phase: DiscoveryPhase, reason: Option<DiscoveryTerminalReason>) {
    metrics::counter!(
        "scanopy_discovery_terminal_total",
        "phase" => phase.to_string(),
        "reason" => reason.map(|r| r.id()).unwrap_or("none"),
    )
    .increment(1);
}

#[async_trait]
impl Subscriber<BillingOperation> for MetricsService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        EventFilter::all()
    }
    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        for event in events {
            record_event("billing", event.operation);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    MetricsService,
    BillingOperation,
>());

#[async_trait]
impl Subscriber<OnboardingOperation> for MetricsService {
    fn filter(&self) -> EventFilter<OnboardingOperation> {
        EventFilter::all()
    }
    async fn handle(&self, events: Vec<Event<OnboardingOperation>>) -> Result<(), Error> {
        for event in events {
            record_event("onboarding", event.operation);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    MetricsService,
    OnboardingOperation,
>());

#[async_trait]
impl Subscriber<AnalyticsOperation> for MetricsService {
    fn filter(&self) -> EventFilter<AnalyticsOperation> {
        EventFilter::all()
    }
    async fn handle(&self, events: Vec<Event<AnalyticsOperation>>) -> Result<(), Error> {
        for event in events {
            record_event("analytics", event.operation);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    MetricsService,
    AnalyticsOperation,
>());

#[async_trait]
impl Subscriber<AuthOperation> for MetricsService {
    fn filter(&self) -> EventFilter<AuthOperation> {
        EventFilter::all()
    }
    async fn handle(&self, events: Vec<Event<AuthOperation>>) -> Result<(), Error> {
        for event in events {
            record_event("auth", event.operation);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<MetricsService, AuthOperation>());

#[async_trait]
impl Subscriber<EntityOperation> for MetricsService {
    fn filter(&self) -> EntityEventFilter {
        EntityEventFilter::all()
    }
    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        for event in events {
            let entity_type = event.scope.entity_type().discriminant().to_string();
            record_entity(&entity_type, event.operation);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<MetricsService, EntityOperation>());

#[async_trait]
impl Subscriber<DiscoveryPhase> for MetricsService {
    fn filter(&self) -> EventFilter<DiscoveryPhase> {
        EventFilter::all()
    }
    async fn handle(&self, events: Vec<Event<DiscoveryPhase>>) -> Result<(), Error> {
        for event in events {
            record_event("discovery", event.operation);
            if event.operation.is_terminal() {
                record_discovery_terminal(event.operation, event.scope.reason);
            }
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<MetricsService, DiscoveryPhase>());

#[async_trait]
impl Subscriber<DiscoveryWarningCode> for MetricsService {
    /// Every code, including `Unknown`. An allowlist here would mean a new code silently missing
    /// from the metric, and `Unknown` in particular has to be counted: it is what an old daemon's
    /// bare-string warnings land as, and dropping those would leave a hole in the numbers exactly
    /// while a fleet is upgrading.
    fn filter(&self) -> EventFilter<DiscoveryWarningCode> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<DiscoveryWarningCode>>) -> Result<(), Error> {
        for event in events {
            record_discovery_warning(event.operation, event.scope.integration);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    MetricsService,
    DiscoveryWarningCode,
>());
