//! Logging subscriber for every operation type.
//!
//! Emits one line per event: a `log_label` field (e.g. `subnet_created`) plus a
//! `log_color` ANSI code, with the event rendered as JSON via
//! `Display for Event<Op>` as the message, at the event's declared `log_level`.
//! The `logging::format::LabelFields` formatter turns those into a color-coded
//! `<label>: <json>` line. Honours `flags.suppress_logs` to keep noisy
//! emissions out of the logs.

use anyhow::Error;
use async_trait::async_trait;

use crate::{
    daemon::discovery::types::{base::DiscoveryPhase, warnings::DiscoveryWarningCode},
    server::{
        logging::service::LoggingService,
        shared::events::{
            registry::SubscriberRegistration,
            traits::{EntityEventFilter, Event, EventFilter, Operation, Subscriber},
            types::{
                AnalyticsOperation, AuthOperation, BillingOperation, EntityOperation,
                EventLogLevel, OnboardingOperation,
            },
        },
        subnets::r#impl::correction_events::SubnetCorrection,
    },
};

fn log_at_level(
    level: EventLogLevel,
    log_label: &str,
    log_color: &str,
    message: impl std::fmt::Display,
) {
    match level {
        EventLogLevel::Error => {
            tracing::error!(target: "events", log_label, log_color, "{}", message)
        }
        EventLogLevel::Warn => {
            tracing::warn!(target: "events", log_label, log_color, "{}", message)
        }
        EventLogLevel::Info => {
            tracing::info!(target: "events", log_label, log_color, "{}", message)
        }
        EventLogLevel::Debug => {
            tracing::debug!(target: "events", log_label, log_color, "{}", message)
        }
        EventLogLevel::Trace => {
            tracing::trace!(target: "events", log_label, log_color, "{}", message)
        }
    }
}

fn log_event<Op: Operation>(event: &Event<Op>, suppress: bool) {
    if suppress {
        return;
    }
    log_at_level(
        event.operation.log_level(),
        &event.log_label(),
        event.operation.log_color().ansi_code(),
        event,
    );
}

#[async_trait]
impl Subscriber<BillingOperation> for LoggingService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    LoggingService,
    BillingOperation,
>());

#[async_trait]
impl Subscriber<OnboardingOperation> for LoggingService {
    fn filter(&self) -> EventFilter<OnboardingOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<OnboardingOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    LoggingService,
    OnboardingOperation,
>());

#[async_trait]
impl Subscriber<AnalyticsOperation> for LoggingService {
    fn filter(&self) -> EventFilter<AnalyticsOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<AnalyticsOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    LoggingService,
    AnalyticsOperation,
>());

#[async_trait]
impl Subscriber<AuthOperation> for LoggingService {
    fn filter(&self) -> EventFilter<AuthOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<AuthOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<LoggingService, AuthOperation>());

#[async_trait]
impl Subscriber<EntityOperation> for LoggingService {
    fn filter(&self) -> EntityEventFilter {
        EntityEventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<LoggingService, EntityOperation>());

#[async_trait]
impl Subscriber<DiscoveryPhase> for LoggingService {
    fn filter(&self) -> EventFilter<DiscoveryPhase> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<DiscoveryPhase>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<LoggingService, DiscoveryPhase>());

#[async_trait]
impl Subscriber<DiscoveryWarningCode> for LoggingService {
    fn filter(&self) -> EventFilter<DiscoveryWarningCode> {
        EventFilter::all()
    }

    /// One line per warning, at `Warn`, with the occurrence's own evidence in it.
    ///
    /// This is what replaced the hand-written `tracing::warn!` calls the LLDP resolver used to
    /// carry: those said the same thing as the warning beside them, and the per-neighbour detail
    /// they added was visible only to whoever had container access. Now the detail is on the
    /// warning itself, so the log and the scan record say the same thing from one source.
    async fn handle(&self, events: Vec<Event<DiscoveryWarningCode>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    LoggingService,
    DiscoveryWarningCode,
>());

#[async_trait]
impl Subscriber<SubnetCorrection> for LoggingService {
    fn filter(&self) -> EventFilter<SubnetCorrection> {
        EventFilter::all()
    }

    /// One line per corrected range, at `Warn`, carrying the range it used to be.
    ///
    /// Subnet rows are updated in place rather than versioned, so once the correction is written
    /// nothing else remembers the old range. This line is the record.
    async fn handle(&self, events: Vec<Event<SubnetCorrection>>) -> Result<(), Error> {
        for event in events {
            log_event(&event, event.flags.suppress_logs);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    LoggingService,
    SubnetCorrection,
>());
