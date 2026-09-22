//! PostHog subscriber for billing, onboarding, analytics, auth, entity, and
//! discovery events.
//!
//! Captures product analytics: emits PostHog `capture` events keyed on
//! distinct_id (user_id when known, else organization_id), updates person
//! properties (plan, status), and groups events under the org.

use crate::{
    daemon::discovery::types::{
        base::{DiscoveryPhase, DiscoveryPhaseDiscriminants},
        warnings::DiscoveryWarningCode,
    },
    server::{
        auth::middleware::auth::AuthenticatedEntity,
        discovery::r#impl::types::DiscoveryType,
        posthog::service::PosthogService,
        shared::{
            events::{
                registry::SubscriberRegistration,
                traits::{EntityEventFilter, Event, EventFilter, Subscriber},
                types::{
                    AnalyticsOperation, AnalyticsOperationDiscriminants, AuthOperation,
                    AuthOperationDiscriminants, BillingOperation, EntityOperation,
                    EntityOperationDiscriminants, OnboardingOperation,
                    OnboardingOperationDiscriminants,
                },
            },
            types::metadata::TypeMetadataProvider,
        },
    },
};
use anyhow::Error;
use async_trait::async_trait;
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;

/// Demo org ID — filtered from noisy analytics events to avoid skewing metrics.
const DEMO_ORG_ID: Uuid = uuid::uuid!("0380451f-a50b-41cd-ae76-6ce47214d8ff");

/// Build common properties from an event's `AuthenticatedEntity`.
fn auth_properties(auth: &AuthenticatedEntity) -> serde_json::Value {
    let mut props = json!({
        "auth_type": auth.entity_name(),
    });
    if let Some(user_id) = auth.user_id() {
        props["user_id"] = json!(user_id.to_string());
    }
    if let Some(email) = auth.email() {
        props["email"] = json!(email.to_string());
    }
    if let Some(org_id) = auth.organization_id() {
        props["organization_id"] = json!(org_id.to_string());
    }
    if let Some(daemon_id) = auth.daemon_id() {
        props["daemon_id"] = json!(daemon_id.to_string());
    }
    props
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

fn inject_org_group(props: &mut serde_json::Value) {
    if let Some(org_id) = props.get("organization_id").and_then(|v| v.as_str()) {
        props["$groups"] = json!({"organization": org_id});
    }
}

impl PosthogService {
    /// Resolve a distinct_id for PostHog. Returns None if the event cannot be
    /// attributed.
    async fn resolve_distinct_id_for_user(
        &self,
        auth: &AuthenticatedEntity,
        org_id: Option<Uuid>,
    ) -> Option<String> {
        if let Some(user_id) = auth.user_id() {
            return Some(user_id.to_string());
        }
        if let Some(org_id) = org_id {
            return Some(format!("org:{}", org_id));
        }
        if let Some(org_id) = auth.organization_id() {
            return Some(format!("org:{}", org_id));
        }
        None
    }

    async fn resolve_distinct_id_via_network(
        &self,
        auth: &AuthenticatedEntity,
        network_id: Uuid,
    ) -> Option<String> {
        if let Some(user_id) = auth.user_id() {
            return Some(user_id.to_string());
        }
        if let Some(org_id) = self.get_org_id_from_network(&network_id).await {
            return Some(format!("org:{}", org_id));
        }
        if let Some(org_id) = auth.organization_id() {
            return Some(format!("org:{}", org_id));
        }
        None
    }
}

#[async_trait]
impl Subscriber<EntityOperation> for PosthogService {
    fn filter(&self) -> EntityEventFilter {
        use crate::server::shared::entities::EntityDiscriminants;
        let create_or_delete = Some(vec![
            EntityOperationDiscriminants::Created,
            EntityOperationDiscriminants::Deleted,
        ]);
        EntityEventFilter::by_entity(std::collections::HashMap::from([
            (EntityDiscriminants::Network, create_or_delete.clone()),
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

    async fn handle(&self, events: Vec<Event<EntityOperation>>) -> Result<(), Error> {
        use strum::IntoDiscriminant;

        for event in events {
            if event.flags.suppress_logs {
                continue;
            }
            let entity_disc = event.scope.entity_type().discriminant();

            let scope_org_id = event.scope.organization_id();
            let scope_network_id = event.scope.network_id();

            let distinct_id = if let Some(network_id) = scope_network_id {
                self.resolve_distinct_id_via_network(&event.authentication, network_id)
                    .await
            } else {
                self.resolve_distinct_id_for_user(&event.authentication, scope_org_id)
                    .await
            };
            let Some(distinct_id) = distinct_id else {
                tracing::debug!(
                    entity_type = %entity_disc,
                    entity_id = %event.scope.entity_id(),
                    "Skipping PostHog entity event — cannot attribute"
                );
                continue;
            };

            let entity_type_str = to_snake_case(entity_disc.as_ref());
            let event_name = format!("{}_{}", entity_type_str, event.operation);

            let mut props = auth_properties(&event.authentication);
            props["entity_id"] = json!(event.scope.entity_id().to_string());
            if let Some(network_id) = scope_network_id {
                props["network_id"] = json!(network_id.to_string());
                if let Some(org_id) = self.get_org_id_from_network(&network_id).await {
                    props["organization_id"] = json!(org_id.to_string());
                }
            }
            if let Some(org_id) = scope_org_id {
                props["organization_id"] = json!(org_id.to_string());
            }

            inject_org_group(&mut props);
            self.capture(&event_name, &distinct_id, props).await;
        }
        Ok(())
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

    async fn handle(&self, events: Vec<Event<AuthOperation>>) -> Result<(), Error> {
        for event in events {
            if event.flags.suppress_logs {
                continue;
            }
            let distinct_id = event
                .scope
                .user_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let mut props = auth_properties(&event.authentication);
            if let Some(org_id) = event.scope.organization_id {
                props["organization_id"] = json!(org_id.to_string());
            }

            inject_org_group(&mut props);
            self.capture("login", &distinct_id, props).await;
        }
        Ok(())
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<PosthogService, AuthOperation>());

#[async_trait]
impl Subscriber<BillingOperation> for PosthogService {
    fn filter(&self) -> EventFilter<BillingOperation> {
        // Forward every billing event. `handle()` is fully generic over the
        // variant (event name from `to_string()`, full payload serialized into
        // `metadata`, plan from `plan()`), so there is no per-variant work to
        // maintain. Matching all (rather than an allowlist of discriminants)
        // means a newly added `BillingOperation` variant can never be silently
        // dropped from analytics — the gap an explicit list invites.
        EventFilter::all()
    }

    async fn handle(&self, events: Vec<Event<BillingOperation>>) -> Result<(), Error> {
        for event in events {
            if event.flags.suppress_logs {
                continue;
            }

            let org_id = event.scope.organization_id;
            let Some(distinct_id) = self
                .resolve_distinct_id_for_user(&event.authentication, Some(org_id))
                .await
            else {
                tracing::debug!(
                    operation = %event.operation,
                    "Skipping PostHog billing event — cannot attribute"
                );
                continue;
            };

            let event_name = event.operation.to_string();
            let org_id_str = org_id.to_string();

            let mut props = auth_properties(&event.authentication);
            props["organization_id"] = json!(&org_id_str);
            props["metadata"] =
                serde_json::to_value(&event.operation).unwrap_or(serde_json::Value::Null);

            inject_org_group(&mut props);
            self.capture(&event_name, &distinct_id, props).await;

            // Update person and group properties. Use the *resulting* plan so a
            // downgrade-to-Free (cancellation / unconverted trial) labels the
            // person/org as Free, not the outgoing paid plan that `plan()`
            // carries.
            let plan_name: serde_json::Value = event
                .operation
                .resulting_plan_name()
                .map(|n| json!(n))
                .unwrap_or(json!(null));

            self.identify(
                &distinct_id,
                json!({
                    "plan_type": plan_name,
                }),
            )
            .await;

            self.group_identify(
                "organization",
                &org_id_str,
                json!({
                    "plan_type": plan_name,
                }),
            )
            .await;
        }
        Ok(())
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
            OnboardingOperationDiscriminants::SecondNetworkCreated,
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
        for event in events {
            if event.flags.suppress_logs {
                continue;
            }
            let org_id = event.scope.organization_id;
            let Some(distinct_id) = self
                .resolve_distinct_id_for_user(&event.authentication, Some(org_id))
                .await
            else {
                tracing::debug!(
                    operation = %event.operation,
                    "Skipping PostHog onboarding event — cannot attribute"
                );
                continue;
            };

            let event_name = event.operation.to_string();
            let org_id_str = org_id.to_string();

            let mut props = auth_properties(&event.authentication);
            props["organization_id"] = json!(&org_id_str);
            props["metadata"] =
                serde_json::to_value(&event.operation).unwrap_or(serde_json::Value::Null);

            inject_org_group(&mut props);
            self.capture(&event_name, &distinct_id, props).await;

            if let OnboardingOperation::OrgCreated {
                org_name,
                plan,
                use_case,
                ..
            } = &event.operation
            {
                let plan_type = json!(plan.name());

                self.identify(
                    &distinct_id,
                    json!({
                        "plan_type": plan_type,
                        "organization_id": &org_id_str,
                        "use_case": use_case,
                    }),
                )
                .await;

                self.group_identify(
                    "organization",
                    &org_id_str,
                    json!({
                        "plan_type": plan_type,
                        "name": org_name,
                        "use_case": use_case,
                        "created_at": event.timestamp.to_rfc3339(),
                    }),
                )
                .await;
            }
        }
        Ok(())
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
        EventFilter::ops(vec![
            AnalyticsOperationDiscriminants::TopologyShareViewed,
            AnalyticsOperationDiscriminants::TopologyEmbedViewed,
        ])
    }

    async fn handle(&self, events: Vec<Event<AnalyticsOperation>>) -> Result<(), Error> {
        for event in events {
            if event.flags.suppress_logs {
                continue;
            }
            let org_id = event.scope.organization_id;
            // Skip share/embed view events from the demo org to avoid skewing metrics
            if org_id == DEMO_ORG_ID
                && matches!(
                    event.operation,
                    AnalyticsOperation::TopologyShareViewed { .. }
                        | AnalyticsOperation::TopologyEmbedViewed { .. }
                )
            {
                continue;
            }

            let distinct_id = format!("org:{}", org_id);
            let event_name = event.operation.to_string();

            let mut props = auth_properties(&event.authentication);
            props["organization_id"] = json!(org_id.to_string());
            if let Ok(serde_json::Value::Object(payload)) = serde_json::to_value(&event.operation) {
                for (k, v) in payload {
                    if k != "type" {
                        props[k] = v;
                    }
                }
            }

            inject_org_group(&mut props);
            self.capture(&event_name, &distinct_id, props).await;
        }
        Ok(())
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

    async fn handle(&self, events: Vec<Event<DiscoveryPhase>>) -> Result<(), Error> {
        for event in events {
            if event.flags.suppress_logs {
                continue;
            }
            let event_name = match event.operation {
                DiscoveryPhase::Pending => "discovery_started",
                DiscoveryPhase::Complete => "discovery_completed",
                DiscoveryPhase::Failed => "discovery_failed",
                DiscoveryPhase::Cancelled => "discovery_cancelled",
                _ => continue,
            };

            let Some(distinct_id) = self
                .resolve_distinct_id_via_network(&event.authentication, event.scope.network_id)
                .await
            else {
                tracing::debug!(
                    session_id = %event.scope.session_id,
                    "Skipping PostHog discovery event — cannot attribute"
                );
                continue;
            };

            let mut props = auth_properties(&event.authentication);
            props["session_id"] = json!(event.scope.session_id.to_string());
            props["network_id"] = json!(event.scope.network_id.to_string());
            props["daemon_id"] = json!(event.scope.daemon_id.to_string());

            let type_name: &'static str = (&event.scope.discovery_type).into();
            props["discovery_type"] = json!(type_name);
            if let DiscoveryType::Network { subnet_ids, .. } = &event.scope.discovery_type {
                props["discovery_subnet_scan"] = json!(subnet_ids.is_some());
            }

            if let Some(error_reason) = &event.scope.error_reason {
                props["error_reason"] = json!(error_reason);
            }
            // Splits stalls from failures the daemon reported, which share `discovery_failed`.
            if let Some(reason) = event.scope.reason {
                props["reason"] = json!(reason);
            }

            if let Some(org_id) = self.get_org_id_from_network(&event.scope.network_id).await {
                props["organization_id"] = json!(org_id.to_string());
            }

            inject_org_group(&mut props);
            self.capture(event_name, &distinct_id, props).await;
        }
        Ok(())
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
    /// rides along as `occurrences`. A session's warnings are all published in one loop, so they
    /// arrive inside one debounce window.
    ///
    /// Nothing that identifies a customer's network reaches PostHog: no address, no host id, and
    /// not the library diagnostic on `detail`.
    async fn handle(&self, events: Vec<Event<DiscoveryWarningCode>>) -> Result<(), Error> {
        // Keyed rather than counted per event so the org lookup below runs once per session
        // instead of once per warning — two DB round-trips times several hundred is not a cost
        // worth paying for a number that would be identical every time.
        let mut grouped: BTreeMap<(Uuid, DiscoveryWarningCode, Option<String>), Grouped> =
            BTreeMap::new();

        for event in events {
            if event.flags.suppress_logs {
                continue;
            }
            let integration = event.scope.integration.map(|i| i.to_string());
            let entry = grouped
                .entry((event.scope.session_id, event.operation, integration))
                .or_insert_with(|| Grouped {
                    occurrences: 0,
                    network_id: event.scope.network_id,
                    daemon_id: event.scope.daemon_id,
                    authentication: event.authentication.clone(),
                });
            entry.occurrences += 1;
        }

        for ((session_id, code, integration), group) in grouped {
            let Some(distinct_id) = self
                .resolve_distinct_id_via_network(&group.authentication, group.network_id)
                .await
            else {
                tracing::debug!(
                    session_id = %session_id,
                    "Skipping PostHog discovery-warning event — cannot attribute"
                );
                continue;
            };

            let mut props = auth_properties(&group.authentication);
            props["session_id"] = json!(session_id.to_string());
            props["network_id"] = json!(group.network_id.to_string());
            props["daemon_id"] = json!(group.daemon_id.to_string());
            props["code"] = json!(code.to_string());
            props["integration"] = json!(integration.unwrap_or_else(|| "none".to_string()));
            props["occurrences"] = json!(group.occurrences);

            if let Some(org_id) = self.get_org_id_from_network(&group.network_id).await {
                props["organization_id"] = json!(org_id.to_string());
            }

            inject_org_group(&mut props);
            self.capture("discovery_warning", &distinct_id, props).await;
        }
        Ok(())
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
}
inventory::submit!(SubscriberRegistration::new::<
    PosthogService,
    DiscoveryWarningCode,
>());

/// One `(session, code, integration)` bucket, and what it takes to attribute it once.
struct Grouped {
    occurrences: usize,
    network_id: Uuid,
    daemon_id: Uuid,
    authentication: AuthenticatedEntity,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("Host"), "host");
        assert_eq!(to_snake_case("DaemonApiKey"), "daemon_api_key");
        assert_eq!(to_snake_case("UserApiKey"), "user_api_key");
        assert_eq!(to_snake_case("Credential"), "credential");
        assert_eq!(to_snake_case("Network"), "network");
        assert_eq!(to_snake_case("Interface"), "interface");
    }

    #[test]
    fn test_inject_org_group() {
        let mut props = json!({
            "organization_id": "abc-123",
            "user_id": "user-456",
        });
        inject_org_group(&mut props);
        assert_eq!(props["$groups"], json!({"organization": "abc-123"}));
    }

    #[test]
    fn test_inject_org_group_no_org() {
        let mut props = json!({
            "user_id": "user-456",
        });
        inject_org_group(&mut props);
        assert_eq!(props.get("$groups"), None);
    }
}
