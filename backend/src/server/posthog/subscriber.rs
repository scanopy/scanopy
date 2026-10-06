//! PostHog subscriber for billing, onboarding, analytics, auth, entity, and
//! discovery events.
//!
//! Captures product analytics. Every event goes out through
//! [`PosthogService::capture_event`] in the shape the event layer defines
//! ([`Event::properties`]): who acted, the server version, and the event's own
//! fields under `metadata`, grouped under its organization. These handlers
//! choose which events to send and update person and group properties (plan,
//! use case) where an event settles them.
//!
//! Each handler sends its whole debounced batch and reports failed sends as `NonRetryable`: a
//! re-run would capture the events that already went through a second time.

use crate::{
    daemon::discovery::types::{
        base::{DiscoveryPhase, DiscoveryPhaseDiscriminants},
        warnings::DiscoveryWarningCode,
    },
    server::{
        organizations::r#impl::base::UseCase,
        posthog::service::PosthogService,
        shared::{
            events::{
                registry::SubscriberRegistration,
                traits::{EntityEventFilter, Event, EventFilter, NonRetryable, Subscriber},
                types::{
                    AnalyticsOperation, AuthOperation, AuthOperationDiscriminants,
                    BillingOperation, EntityOperation, EntityOperationDiscriminants,
                    OnboardingOperation, OnboardingOperationDiscriminants,
                },
            },
            types::metadata::TypeMetadataProvider,
        },
    },
};
use anyhow::Error;
use async_trait::async_trait;
use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;

/// Demo org ID — filtered from noisy analytics events to avoid skewing metrics.
const DEMO_ORG_ID: Uuid = uuid::uuid!("0380451f-a50b-41cd-ae76-6ce47214d8ff");

/// Person properties an event sets when it settles the org's plan.
#[derive(Serialize)]
struct PersonProperties<'a> {
    plan_type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    organization_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_case: Option<&'a UseCase>,
}

/// Organization group properties an event sets when it settles the org's plan.
#[derive(Serialize)]
struct OrgGroupProperties<'a> {
    plan_type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_case: Option<&'a UseCase>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
}

/// A warning bucket's metadata plus how many warnings it collapsed.
#[derive(Serialize)]
struct WithOccurrences<M> {
    #[serde(flatten)]
    inner: M,
    occurrences: usize,
}

impl PosthogService {
    /// Capture each event, collecting failed sends.
    async fn capture_all<Op: crate::server::shared::events::Operation>(
        &self,
        events: &[Event<Op>],
    ) -> Vec<Error> {
        let mut failures = Vec::new();
        for event in events {
            if let Some((_, result)) = self.capture_event(event).await {
                failures.extend(result.err());
            }
        }
        failures
    }
}

fn failures_to_result(failures: Vec<Error>) -> Result<(), Error> {
    NonRetryable::from_failures(failures).map_or(Ok(()), |e| Err(e.into()))
}

fn entity_filter() -> EntityEventFilter {
    use crate::server::shared::entities::EntityDiscriminants;
    let create_or_delete = Some(vec![
        EntityOperationDiscriminants::Created,
        EntityOperationDiscriminants::Deleted,
    ]);
    EntityEventFilter::by_entity(std::collections::HashMap::from([
        // Deleted only: creation is already `org_created` (onboarding). Deleting an org deletes
        // its Stripe customer and the cancellation webhook then finds no org, so without this a
        // trial org that deletes itself leaves no outcome event.
        (
            EntityDiscriminants::Organization,
            Some(vec![EntityOperationDiscriminants::Deleted]),
        ),
        (EntityDiscriminants::Site, create_or_delete.clone()),
        (EntityDiscriminants::Host, create_or_delete.clone()),
        (EntityDiscriminants::Subnet, create_or_delete.clone()),
        (EntityDiscriminants::Discovery, create_or_delete.clone()),
        (EntityDiscriminants::Dependency, create_or_delete.clone()),
        (EntityDiscriminants::Tag, create_or_delete.clone()),
        (EntityDiscriminants::Share, create_or_delete.clone()),
        (EntityDiscriminants::Vlan, create_or_delete.clone()),
        (EntityDiscriminants::UserApiKey, create_or_delete.clone()),
        (EntityDiscriminants::DaemonApiKey, create_or_delete.clone()),
        (EntityDiscriminants::Daemon, create_or_delete.clone()),
        (EntityDiscriminants::Credential, create_or_delete.clone()),
        (EntityDiscriminants::Invite, create_or_delete.clone()),
        (EntityDiscriminants::User, create_or_delete),
    ]))
}

#[async_trait]
impl Subscriber<EntityOperation> for PosthogService {
    fn filter(&self) -> EntityEventFilter {
        entity_filter()
    }

    async fn handle(&self, mut events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        events.retain(|e| !e.flags.suppress_logs);
        failures_to_result(self.capture_all(&events).await)
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<PosthogService, EntityOperation>());

#[async_trait]
impl Subscriber<AuthOperation> for PosthogService {
    fn filter(&self) -> EventFilter<AuthOperation> {
        EventFilter::ops(vec![AuthOperationDiscriminants::LoginSuccess])
    }

    async fn handle(&self, mut events: Vec<Event<AuthOperation>>) -> Result<(), Error> {
        events.retain(|e| !e.flags.suppress_logs);
        failures_to_result(self.capture_all(&events).await)
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<PosthogService, AuthOperation>());

#[async_trait]
impl Subscriber<BillingOperation> for PosthogService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        // Forward every billing event. The name, `metadata` and plan all come
        // generically from the operation, so there is no per-variant work to
        // maintain. Matching all (rather than an allowlist of discriminants)
        // means a newly added `BillingOperation` variant can never be silently
        // dropped from analytics — the gap an explicit list invites.
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        let mut failures = Vec::new();
        for event in events {
            if event.flags.suppress_logs {
                continue;
            }
            if event.operation.is_zero_dollar_notice() {
                tracing::debug!(
                    operation = %event.operation,
                    organization_id = %event.scope.organization_id,
                    "Skipping PostHog billing event for a customer paying nothing"
                );
                continue;
            }

            let Some((distinct_id, result)) = self.capture_event(&event).await else {
                continue;
            };
            failures.extend(result.err());

            // Update person and group properties from the plan the org lands
            // on. Events that carry no plan (payment method, invoice, discount
            // events and the like) leave `plan_type` as it is rather than
            // nulling it.
            let Some(plan_type) = event.operation.resulting_plan_name() else {
                continue;
            };
            let person = PersonProperties {
                plan_type,
                organization_id: None,
                use_case: None,
            };
            let group = OrgGroupProperties {
                plan_type,
                name: None,
                use_case: None,
                created_at: None,
            };
            failures.extend(self.identify(&distinct_id, &person).await.err());
            failures.extend(
                self.group_identify(
                    "organization",
                    &event.scope.organization_id.to_string(),
                    &group,
                )
                .await
                .err(),
            );
        }
        failures_to_result(failures)
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<
    PosthogService,
    BillingOperation,
>());

#[async_trait]
impl Subscriber<OnboardingOperation> for PosthogService {
    fn filter(&self) -> EventFilter<OnboardingOperation> {
        EventFilter::ops(vec![
            OnboardingOperationDiscriminants::OrgCreated,
            OnboardingOperationDiscriminants::OnboardingModalCompleted,
            OnboardingOperationDiscriminants::PlanSelected,
            OnboardingOperationDiscriminants::DaemonPromptDismissed,
            OnboardingOperationDiscriminants::DaemonPromptAccepted,
            OnboardingOperationDiscriminants::FirstDaemonRegistered,
            OnboardingOperationDiscriminants::FirstTopologyRebuild,
            OnboardingOperationDiscriminants::FirstDiscoveryCompleted,
            OnboardingOperationDiscriminants::FirstHostDiscovered,
            OnboardingOperationDiscriminants::SecondSiteCreated,
            OnboardingOperationDiscriminants::FirstTagCreated,
            OnboardingOperationDiscriminants::FirstDependencyCreated,
            OnboardingOperationDiscriminants::FirstUserApiKeyCreated,
            OnboardingOperationDiscriminants::FirstSnmpCredentialCreated,
            OnboardingOperationDiscriminants::FirstCredentialCreated,
            OnboardingOperationDiscriminants::InviteSent,
            OnboardingOperationDiscriminants::InviteAccepted,
            OnboardingOperationDiscriminants::ProfileCompleted,
            OnboardingOperationDiscriminants::FirstApplicationTagCreated,
            OnboardingOperationDiscriminants::FirstSnapshotCreated,
            OnboardingOperationDiscriminants::ReferralSourceCompleted,
        ])
    }

    async fn handle(&self, events: Vec<Event<OnboardingOperation>>) -> Result<(), Error> {
        let mut failures = Vec::new();
        for event in events {
            if event.flags.suppress_logs {
                continue;
            }
            let Some((distinct_id, result)) = self.capture_event(&event).await else {
                continue;
            };
            failures.extend(result.err());

            if let OnboardingOperation::OrgCreated {
                org_name,
                plan,
                use_case,
                ..
            } = &event.operation
            {
                let org_id = event.scope.organization_id;
                let person = PersonProperties {
                    plan_type: plan.name(),
                    organization_id: Some(org_id),
                    use_case: Some(use_case),
                };
                failures.extend(self.identify(&distinct_id, &person).await.err());

                let group = OrgGroupProperties {
                    plan_type: plan.name(),
                    name: Some(org_name),
                    use_case: Some(use_case),
                    created_at: Some(event.timestamp.to_rfc3339()),
                };
                failures.extend(
                    self.group_identify("organization", &org_id.to_string(), &group)
                        .await
                        .err(),
                );
            }
        }
        failures_to_result(failures)
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<
    PosthogService,
    OnboardingOperation,
>());

#[async_trait]
impl Subscriber<AnalyticsOperation> for PosthogService {
    fn filter(&self) -> EventFilter<AnalyticsOperation> {
        // Forward every analytics event: the person comes from the
        // operation's `attribution`, an exhaustive match, and the payload is
        // serialized generically into `metadata`, so a new variant needs no
        // change here. Same reasoning as the billing filter.
        EventFilter::all()
    }

    async fn handle(&self, mut events: Vec<Event<AnalyticsOperation>>) -> Result<(), Error> {
        // Skip the demo org's share views and emails to avoid skewing metrics
        events.retain(|e| !e.flags.suppress_logs && e.scope.organization_id != DEMO_ORG_ID);
        failures_to_result(self.capture_all(&events).await)
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<
    PosthogService,
    AnalyticsOperation,
>());

#[async_trait]
impl Subscriber<DiscoveryPhase> for PosthogService {
    fn filter(&self) -> EventFilter<DiscoveryPhase> {
        EventFilter::ops(vec![
            DiscoveryPhaseDiscriminants::Pending,
            DiscoveryPhaseDiscriminants::Complete,
            DiscoveryPhaseDiscriminants::Failed,
            DiscoveryPhaseDiscriminants::Cancelled,
        ])
    }

    async fn handle(&self, mut events: Vec<Event<DiscoveryPhase>>) -> Result<(), Error> {
        events.retain(|e| !e.flags.suppress_logs);
        failures_to_result(self.capture_all(&events).await)
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<PosthogService, DiscoveryPhase>());

#[async_trait]
impl Subscriber<DiscoveryWarningCode> for PosthogService {
    /// Every code, `Unknown` included. An allowlist is what lets a newly added code go silently
    /// missing from analytics — the argument the billing subscriber above makes, and it holds
    /// harder here, where the codes are the whole point.
    fn filter(&self) -> EventFilter<DiscoveryWarningCode> {
        EventFilter::all()
    }

    /// One event per `(session, code, integration)`, not per occurrence.
    ///
    /// Warnings are recorded one per affected device, so a single scan can raise hundreds. Product
    /// analytics is asking what fraction of scans hit a given failure mode, not how many devices
    /// each hit — Grafana already counts devices — so the batch is collapsed and the device count
    /// rides along as `metadata.occurrences`. A session's warnings are all published in one loop,
    /// so they arrive inside one debounce window.
    ///
    /// The bucket is sent as its first event: every event in it has the same session, code and
    /// integration, and the warning's own metadata carries nothing that varies by device.
    async fn handle(&self, events: Vec<Event<DiscoveryWarningCode>>) -> Result<(), Error> {
        // Keyed rather than sent per event so the org lookup runs once per bucket instead of
        // once per warning — a DB round-trip times several hundred is not a cost worth paying
        // for a number that would be identical every time.
        let mut grouped: BTreeMap<(Uuid, DiscoveryWarningCode, Option<String>), Grouped> =
            BTreeMap::new();
        for event in events {
            if event.flags.suppress_logs {
                continue;
            }
            let integration = event.scope.integration.map(|i| i.to_string());
            grouped
                .entry((event.scope.session_id, event.operation, integration))
                .or_insert_with(|| Grouped {
                    occurrences: 0,
                    first: event,
                })
                .occurrences += 1;
        }

        let mut failures = Vec::new();
        for group in grouped.into_values() {
            let properties = self
                .event_properties(&group.first)
                .await
                .map_metadata(|inner| WithOccurrences {
                    inner,
                    occurrences: group.occurrences,
                });
            if let Some((_, result)) = self
                .capture_as(&group.first.name(), group.first.attribution(), &properties)
                .await
            {
                failures.extend(result.err());
            }
        }
        failures_to_result(failures)
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<
    PosthogService,
    DiscoveryWarningCode,
>());

/// One `(session, code, integration)` bucket: its first event and how many it collapsed.
struct Grouped {
    occurrences: usize,
    first: Event<DiscoveryWarningCode>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{
        auth::middleware::auth::AuthenticatedEntity,
        posthog::service::{distinct_id, to_posthog_event},
        shared::events::{operations::tests::samples, traits::OrgScope},
    };
    use serde_json::{Value, json};

    /// What PostHog receives: properties, plus the `$groups` posthog-rs adds from `add_group`.
    fn wire(
        name: &str,
        properties: &crate::server::shared::events::EventProperties<Value>,
    ) -> Value {
        serde_json::to_value(to_posthog_event(name, "someone", properties).expect("builds"))
            .expect("serializes")
    }

    /// Every operation type, sent the one way, joins its organization's group and carries its
    /// own fields under `metadata`.
    #[test]
    fn every_event_is_grouped_on_its_organization_with_metadata() {
        let organization_id = Uuid::new_v4();
        for sample in samples(organization_id) {
            let sent = wire(&sample.name, &sample.properties);
            assert_eq!(
                sent["groups"]["organization"],
                json!(organization_id.to_string()),
                "{} is not grouped on its organization",
                sample.name
            );
            assert!(
                sent["properties"]["metadata"].is_object(),
                "{} has no metadata object",
                sample.name
            );
            assert!(
                distinct_id(sample.attribution, &sample.properties.actor).is_some(),
                "{} lands on no PostHog person",
                sample.name
            );
        }
    }

    #[test]
    fn an_event_with_no_organization_joins_no_group() {
        let organization_id = Uuid::new_v4();
        let mut sample = samples(organization_id).remove(0);
        sample.properties.actor.organization_id = None;
        let sent = wire(&sample.name, &sample.properties);
        assert_eq!(sent["groups"], json!({}));
    }

    #[test]
    fn org_deletion_reaches_posthog_but_org_creation_does_not() {
        use crate::server::shared::events::traits::{EntityScope, SubscriberFilter};
        use crate::server::shared::types::examples;

        // The scope `OrganizationService` builds: an org is scoped to its own id.
        let org = examples::organization();
        let event = |op| {
            let scope = EntityScope::from_ids(org.id, org.clone().into(), None, Some(org.id))
                .expect("org-scoped");
            Event::new(scope, op, AuthenticatedEntity::System)
        };

        assert!(entity_filter().matches(&event(EntityOperation::Deleted)));
        // Creation is `org_created` from onboarding; forwarding it too would double-count.
        assert!(!entity_filter().matches(&event(EntityOperation::Created)));
    }

    /// An `email_sent` event names its email the way billing events name their details: under
    /// `metadata`, where the campaign identifies which email went out.
    #[test]
    fn an_email_send_carries_its_campaign_in_metadata() {
        let organization_id = Uuid::new_v4();
        let event = Event::new(
            OrgScope { organization_id },
            AnalyticsOperation::EmailSent {
                utm_campaign: "trial_ending".to_string(),
                utm_medium: "billing".to_string(),
                user_id: Uuid::new_v4(),
            },
            AuthenticatedEntity::System,
        );

        let properties = serde_json::to_value(event.properties(None)).expect("serializes");
        assert_eq!(
            properties["metadata"]["utm_campaign"],
            json!("trial_ending")
        );
        assert_eq!(properties["metadata"]["utm_medium"], json!("billing"));
    }

    /// A send and the click it produces have to land on the same PostHog
    /// person; share views have no viewer and stay on the org.
    #[test]
    fn an_email_send_belongs_to_its_recipient_and_a_share_view_to_the_org() {
        let organization_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let event = |op| {
            Event::new(
                OrgScope { organization_id },
                op,
                AuthenticatedEntity::System,
            )
        };
        let person = |event: &Event<AnalyticsOperation>| {
            distinct_id(event.attribution(), &event.properties(None).actor)
        };

        let sent = event(AnalyticsOperation::EmailSent {
            utm_campaign: "self_hosted_welcome".to_string(),
            utm_medium: "billing".to_string(),
            user_id,
        });
        let viewed = event(AnalyticsOperation::TopologyShareViewed {
            share_id: Uuid::new_v4(),
            has_password: false,
        });

        assert_eq!(person(&sent), Some(user_id.to_string()));
        assert_eq!(person(&viewed), Some(format!("org:{organization_id}")));
    }
}
