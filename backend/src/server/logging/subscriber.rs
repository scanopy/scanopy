//! Logging subscriber for every operation type.
//!
//! Writes each event as its `EventLog` says: a `log_label` field (e.g. `subnet_created`) plus a
//! `log_color` ANSI code, with a JSON body at the event's declared `log_level`: the whole event,
//! or for an update only the fields that changed, or nothing. A readable line, when the event has
//! one, follows at its own level. The `logging::format::LabelFields` formatter turns those into a
//! color-coded `<label>: <message>` line.

use anyhow::Error;
use async_trait::async_trait;

use crate::{
    daemon::discovery::types::{base::DiscoveryPhase, warnings::DiscoveryWarningCode},
    server::{
        dependencies::r#impl::member_events::DependencyMemberChange,
        logging::service::LoggingService,
        setup::types::SetupOperation,
        shared::events::{
            log::LogBody,
            registry::SubscriberRegistration,
            traits::{EntityEventFilter, Event, EventFilter, Operation, Subscriber},
            types::{
                AnalyticsOperation, AuthOperation, BillingOperation, EntityOperation, EventLogLevel,
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

/// Writes the event as its [`EventLog`](crate::server::shared::events::log::EventLog) says: the
/// body at the operation's level, then any line at its own.
fn log_event<Op: Operation>(event: &Event<Op>) {
    let label = event.log_label();
    let color = event.operation.log_color().ansi_code();
    let level = event.operation.log_level();
    match &event.log.body {
        LogBody::Full => log_at_level(level, &label, color, event),
        LogBody::Changes(changes) => log_at_level(level, &label, color, changes),
        LogBody::Hidden => {}
    }
    if let Some(line) = &event.log.line {
        log_at_level(line.level, &label, color, &line.text);
    }
}

#[async_trait]
impl Subscriber<BillingOperation> for LoggingService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    LoggingService,
    BillingOperation,
>());

#[async_trait]
impl Subscriber<SetupOperation> for LoggingService {
    fn filter(&self) -> EventFilter<SetupOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<SetupOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<LoggingService, SetupOperation>());

#[async_trait]
impl Subscriber<AnalyticsOperation> for LoggingService {
    fn filter(&self) -> EventFilter<AnalyticsOperation> {
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<AnalyticsOperation>>) -> Result<(), Error> {
        for event in events {
            log_event(&event);
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
            log_event(&event);
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
            log_event(&event);
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
            log_event(&event);
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
            log_event(&event);
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
            log_event(&event);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    LoggingService,
    SubnetCorrection,
>());

#[async_trait]
impl Subscriber<DependencyMemberChange> for LoggingService {
    fn filter(&self) -> EventFilter<DependencyMemberChange> {
        EventFilter::all()
    }

    /// One line per member at `Warn`: the dependency is short a member until the user replaces it.
    async fn handle(&self, events: Vec<Event<DependencyMemberChange>>) -> Result<(), Error> {
        for event in events {
            log_event(&event);
        }
        Ok(())
    }
}
inventory::submit!(SubscriberRegistration::new::<
    LoggingService,
    DependencyMemberChange,
>());

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing::{Level, field::Field};
    use tracing_subscriber::{Registry, layer::Context, layer::SubscriberExt};
    use uuid::Uuid;

    use super::*;
    use crate::server::{
        auth::middleware::auth::AuthenticatedEntity,
        billing::plans::get_free_plan,
        shared::{
            entities::Entity,
            events::{
                log::{EventLog, changes_log},
                traits::{AuthScope, EntityScope, OrgScope},
                types::{AuthMethod, DeletionReason, UpdateReason},
            },
        },
    };

    /// Every line the subscriber writes, as its level and message.
    #[derive(Clone, Default)]
    struct Captured(Arc<Mutex<Vec<(Level, String)>>>);

    struct Message(String);

    impl tracing::field::Visit for Message {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.0 = format!("{value:?}");
            }
        }
    }

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Captured {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let mut message = Message(String::new());
            event.record(&mut message);
            self.0
                .lock()
                .unwrap()
                .push((*event.metadata().level(), message.0));
        }
    }

    fn lines_for<Op: Operation>(event: &Event<Op>) -> Vec<(Level, String)> {
        let captured = Captured::default();
        let subscriber = Registry::default().with(captured.clone());
        tracing::subscriber::with_default(subscriber, || log_event(event));
        captured.0.lock().unwrap().clone()
    }

    /// A delete gets a second line, at the event's level, naming the entity and why it went.
    #[test]
    fn a_delete_is_logged_with_a_line_saying_why() {
        let entity_id = Uuid::new_v4();
        let destination = Uuid::new_v4();
        let event = Event::new(
            EntityScope::Site {
                site_id: Uuid::new_v4(),
                entity_id,
                entity_type: Entity::Host(Default::default()),
            },
            EntityOperation::Deleted {
                reason: DeletionReason::HostMergedInto {
                    host_id: destination,
                    proven_by: vec!["10.0.0.7".parse().unwrap()],
                },
            },
            AuthenticatedEntity::System,
        );

        let lines = lines_for(&event);
        assert_eq!(lines.len(), 2, "{lines:?}");
        let (level, detail) = &lines[1];
        assert_eq!(*level, Level::INFO);
        for fact in [
            entity_id.to_string(),
            destination.to_string(),
            "10.0.0.7".to_string(),
        ] {
            assert!(detail.contains(&fact), "{detail}");
        }
    }

    /// Stripe ids kept off a billing event's JSON still reach the log, on the readable line.
    #[test]
    fn log_only_billing_fields_reach_the_log_and_not_the_json() {
        let event = Event::new(
            OrgScope {
                organization_id: Uuid::new_v4(),
            },
            BillingOperation::CheckoutStarted {
                plan: get_free_plan(),
                has_trial: false,
                auto_started: false,
                stripe_id: "cs_test_log_only".to_string(),
                subscription_status: None,
            },
            AuthenticatedEntity::System,
        );

        let lines = lines_for(&event);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(!lines[0].1.contains("cs_test_log_only"), "{}", lines[0].1);
        assert_eq!(lines[1].0, Level::INFO);
        assert!(lines[1].1.contains("cs_test_log_only"), "{}", lines[1].1);
    }

    /// An event without a readable detail writes its JSON line only.
    #[test]
    fn an_event_without_detail_writes_one_line() {
        let event = Event::new(
            OrgScope {
                organization_id: Uuid::new_v4(),
            },
            BillingOperation::PaymentMethodAdded,
            AuthenticatedEntity::System,
        );
        assert_eq!(lines_for(&event).len(), 1);
    }

    /// A failed login says why on a second line at WARN; the reason stays out of the JSON.
    #[test]
    fn a_failed_login_logs_its_reason_on_its_own_line() {
        let event = Event::new(
            AuthScope {
                user_id: None,
                organization_id: None,
                ip_address: "127.0.0.1".parse().unwrap(),
                user_agent: None,
            },
            AuthOperation::LoginFailed {
                method: AuthMethod::Password,
                attempted_email: None,
                reason: "no-account-marker".to_string(),
            },
            AuthenticatedEntity::Anonymous,
        );

        let lines = lines_for(&event);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(!lines[0].1.contains("no-account-marker"), "{}", lines[0].1);
        assert_eq!(lines[1].0, Level::WARN);
        assert!(lines[1].1.contains("no-account-marker"), "{}", lines[1].1);
    }

    fn host_updated() -> Event<EntityOperation> {
        Event::new(
            EntityScope::Site {
                site_id: Uuid::new_v4(),
                entity_id: Uuid::new_v4(),
                entity_type: Entity::Host(Default::default()),
            },
            EntityOperation::Updated {
                reason: UpdateReason::EntityRequested,
            },
            AuthenticatedEntity::System,
        )
    }

    /// An update whose log is its changes writes those, not the entity it carries.
    #[test]
    fn an_update_logged_as_changes_writes_only_the_changes() {
        let before = serde_json::json!({ "name": "before-name", "note": "unchanged-marker" });
        let after = serde_json::json!({ "name": "after-name", "note": "unchanged-marker" });
        let event = host_updated().with_log(changes_log(
            &before,
            &after,
            Uuid::new_v4(),
            &UpdateReason::EntityRequested,
            &[],
        ));

        let lines = lines_for(&event);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].1.contains("after-name"), "{}", lines[0].1);
        assert!(lines[0].1.contains("before-name"), "{}", lines[0].1);
        assert!(!lines[0].1.contains("unchanged-marker"), "{}", lines[0].1);
    }

    /// A hidden update writes nothing, and a line is written at its own level, not the event's.
    #[test]
    fn a_line_keeps_its_own_level_and_a_hidden_body_writes_nothing() {
        assert!(lines_for(&host_updated().with_log(EventLog::hidden())).is_empty());

        let event = host_updated()
            .with_log(EventLog::hidden().with_line(EventLogLevel::Warn, "transition"));
        assert_eq!(
            lines_for(&event),
            vec![(Level::WARN, "transition".to_string())]
        );
    }
}
