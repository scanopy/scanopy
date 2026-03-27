use crate::{
    daemon::discovery::types::base::DiscoveryPhase,
    server::{
        discovery::r#impl::types::DiscoveryType,
        posthog::service::PosthogService,
        shared::{
            entities::EntityDiscriminants,
            events::{
                bus::{EventFilter, EventSubscriber},
                types::{
                    AnalyticsOperation, AuthOperation, BillingOperation, EntityOperation, Event,
                    OnboardingOperation,
                },
            },
        },
    },
};
use anyhow::Error;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

/// Demo org ID — filtered from noisy analytics events to avoid skewing metrics.
const DEMO_ORG_ID: Uuid = uuid::uuid!("0380451f-a50b-41cd-ae76-6ce47214d8ff");

/// Build common properties from the event's authentication context.
fn auth_properties(event: &Event) -> serde_json::Value {
    let auth = event.authentication();
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

/// Convert a PascalCase entity discriminant name to snake_case.
/// e.g. "DaemonApiKey" -> "daemon_api_key", "Host" -> "host"
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

/// Inject `$groups` property for PostHog group analytics when `organization_id` is present.
fn inject_org_group(props: &mut serde_json::Value) {
    if let Some(org_id) = props.get("organization_id").and_then(|v| v.as_str()) {
        props["$groups"] = json!({"organization": org_id});
    }
}

impl PosthogService {
    /// Resolve a distinct_id for PostHog. Returns None if the event cannot be
    /// attributed to a user or organization — caller should skip sending it.
    async fn resolve_distinct_id(&self, event: &Event) -> Option<String> {
        // 1. User/ApiKey auth → user_id
        if let Some(user_id) = event.authentication().user_id() {
            return Some(user_id.to_string());
        }
        // 2. Event has org_id (Telemetry always does) → org:{org_id}
        if let Some(org_id) = event.org_id() {
            return Some(format!("org:{}", org_id));
        }
        // 3. Event has network_id → resolve org via network service → org:{org_id}
        if let Some(network_id) = event.network_id()
            && let Some(org_id) = self.get_org_id_from_network(&network_id).await
        {
            return Some(format!("org:{}", org_id));
        }
        None
    }
}

#[async_trait]
impl EventSubscriber for PosthogService {
    fn event_filter(&self) -> EventFilter {
        let ops = Some(vec![EntityOperation::Created, EntityOperation::Deleted]);
        let mut entity_ops = HashMap::new();
        entity_ops.insert(EntityDiscriminants::Network, ops.clone());
        entity_ops.insert(EntityDiscriminants::Host, ops.clone());
        entity_ops.insert(EntityDiscriminants::Subnet, ops.clone());
        entity_ops.insert(EntityDiscriminants::Discovery, ops.clone());
        entity_ops.insert(EntityDiscriminants::Group, ops.clone());
        entity_ops.insert(EntityDiscriminants::Tag, ops.clone());
        entity_ops.insert(EntityDiscriminants::Share, ops.clone());
        entity_ops.insert(EntityDiscriminants::UserApiKey, ops.clone());
        entity_ops.insert(EntityDiscriminants::DaemonApiKey, ops.clone());
        entity_ops.insert(EntityDiscriminants::Daemon, ops.clone());
        entity_ops.insert(EntityDiscriminants::Credential, ops.clone());
        entity_ops.insert(EntityDiscriminants::Invite, ops.clone());
        entity_ops.insert(EntityDiscriminants::User, ops);

        EventFilter {
            entity_operations: Some(entity_ops),
            auth_operations: Some(vec![AuthOperation::LoginSuccess]),
            billing_operations: Some(vec![
                BillingOperation::CheckoutStarted,
                BillingOperation::CheckoutCompleted,
                BillingOperation::TrialStarted,
                BillingOperation::TrialEnded,
                BillingOperation::TrialWillEnd,
                BillingOperation::SubscriptionCancelled,
                BillingOperation::PlanChanged,
                BillingOperation::PaymentFailed,
                BillingOperation::PaymentActionRequired,
                BillingOperation::PaymentRecovered,
                BillingOperation::FeatureLimitHit,
            ]),
            onboarding_operations: Some(vec![
                OnboardingOperation::OrgCreated,
                OnboardingOperation::OnboardingModalCompleted,
                OnboardingOperation::PlanSelected,
                OnboardingOperation::FirstDaemonRegistered,
                OnboardingOperation::FirstTopologyRebuild,
                OnboardingOperation::FirstDiscoveryCompleted,
                OnboardingOperation::FirstHostDiscovered,
                OnboardingOperation::SecondNetworkCreated,
                OnboardingOperation::FirstTagCreated,
                OnboardingOperation::FirstGroupCreated,
                OnboardingOperation::FirstUserApiKeyCreated,
                OnboardingOperation::FirstSnmpCredentialCreated,
                OnboardingOperation::FirstCredentialCreated,
                OnboardingOperation::InviteSent,
                OnboardingOperation::InviteAccepted,
            ]),
            discovery_phases: Some(vec![
                DiscoveryPhase::Pending,
                DiscoveryPhase::Complete,
                DiscoveryPhase::Failed,
                DiscoveryPhase::Cancelled,
            ]),
            analytics_operations: Some(vec![
                AnalyticsOperation::TopologyShareViewed,
                AnalyticsOperation::TopologyEmbedViewed,
            ]),
            network_ids: None,
        }
    }

    async fn handle_events(&self, events: Vec<Event>) -> Result<(), Error> {
        for event in &events {
            // Skip events with suppress_logs metadata (heartbeat-style updates)
            if event.metadata().get("suppress_logs") == Some(&serde_json::Value::Bool(true)) {
                continue;
            }

            match event {
                Event::Entity(entity_event) => {
                    let Some(distinct_id) = self.resolve_distinct_id(event).await else {
                        tracing::debug!(
                            entity_type = %entity_event.entity_type,
                            entity_id = %entity_event.entity_id,
                            "Skipping PostHog entity event — cannot attribute"
                        );
                        continue;
                    };

                    let entity_type = to_snake_case(&entity_event.entity_type.to_string());
                    let event_name = format!("{}_{}", entity_type, entity_event.operation);

                    let mut props = auth_properties(event);
                    props["entity_id"] = json!(entity_event.entity_id.to_string());
                    if let Some(network_id) = entity_event.network_id {
                        props["network_id"] = json!(network_id.to_string());
                        // Resolve org_id from network if not already present
                        if entity_event.organization_id.is_none()
                            && let Some(org_id) = self.get_org_id_from_network(&network_id).await
                        {
                            props["organization_id"] = json!(org_id.to_string());
                        }
                    }
                    if let Some(org_id) = entity_event.organization_id {
                        props["organization_id"] = json!(org_id.to_string());
                    }

                    inject_org_group(&mut props);
                    self.capture(&event_name, &distinct_id, props).await;
                }
                Event::Auth(auth_event) => {
                    let distinct_id = auth_event
                        .user_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    let mut props = auth_properties(event);
                    if let Some(org_id) = auth_event.organization_id {
                        props["organization_id"] = json!(org_id.to_string());
                    }

                    inject_org_group(&mut props);
                    self.capture("login", &distinct_id, props).await;
                }
                Event::Billing(billing_event) => {
                    let Some(distinct_id) = self.resolve_distinct_id(event).await else {
                        tracing::debug!(
                            operation = %billing_event.operation,
                            "Skipping PostHog billing event — cannot attribute"
                        );
                        continue;
                    };

                    let event_name = billing_event.operation.to_string();
                    let org_id_str = billing_event.organization_id.to_string();

                    let mut props = auth_properties(event);
                    props["organization_id"] = json!(&org_id_str);
                    props["metadata"] = billing_event.metadata.clone();

                    inject_org_group(&mut props);
                    self.capture(&event_name, &distinct_id, props).await;

                    // Update person and group properties on billing events
                    let plan_name = billing_event
                        .metadata
                        .get("plan_name")
                        .cloned()
                        .unwrap_or(json!(null));
                    let plan_status = billing_event
                        .metadata
                        .get("plan_status")
                        .cloned()
                        .unwrap_or(json!(null));
                    let has_payment_method = billing_event
                        .metadata
                        .get("has_payment_method")
                        .cloned()
                        .unwrap_or(json!(null));

                    self.identify(
                        &distinct_id,
                        json!({
                            "plan_type": plan_name,
                            "plan_status": plan_status,
                            "has_payment_method": has_payment_method,
                        }),
                    )
                    .await;

                    self.group_identify(
                        "organization",
                        &org_id_str,
                        json!({
                            "plan_type": plan_name,
                            "plan_status": plan_status,
                        }),
                    )
                    .await;
                }
                Event::Onboarding(onboarding_event) => {
                    let Some(distinct_id) = self.resolve_distinct_id(event).await else {
                        tracing::debug!(
                            operation = %onboarding_event.operation,
                            "Skipping PostHog onboarding event — cannot attribute"
                        );
                        continue;
                    };

                    let event_name = onboarding_event.operation.to_string();
                    let org_id_str = onboarding_event.organization_id.to_string();

                    let mut props = auth_properties(event);
                    props["organization_id"] = json!(&org_id_str);
                    props["metadata"] = onboarding_event.metadata.clone();

                    inject_org_group(&mut props);
                    self.capture(&event_name, &distinct_id, props).await;

                    // Identify person and group on OrgCreated
                    if onboarding_event.operation == OnboardingOperation::OrgCreated {
                        let plan_type = onboarding_event
                            .metadata
                            .get("plan_type")
                            .cloned()
                            .unwrap_or(json!(null));
                        let plan_status = onboarding_event
                            .metadata
                            .get("plan_status")
                            .cloned()
                            .unwrap_or(json!(null));
                        let has_payment_method = onboarding_event
                            .metadata
                            .get("has_payment_method")
                            .cloned()
                            .unwrap_or(json!(false));
                        let org_name = onboarding_event
                            .metadata
                            .get("org_name")
                            .cloned()
                            .unwrap_or(json!(null));
                        let use_case = onboarding_event
                            .metadata
                            .get("use_case")
                            .cloned()
                            .unwrap_or(json!(null));

                        self.identify(
                            &distinct_id,
                            json!({
                                "plan_type": plan_type,
                                "plan_status": plan_status,
                                "has_payment_method": has_payment_method,
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
                                "plan_status": plan_status,
                                "name": org_name,
                                "use_case": use_case,
                                "created_at": onboarding_event.timestamp.to_rfc3339(),
                            }),
                        )
                        .await;
                    }
                }
                Event::Analytics(analytics_event) => {
                    // Skip share/embed view events from the demo org to avoid skewing metrics
                    if analytics_event.organization_id == DEMO_ORG_ID
                        && matches!(
                            analytics_event.operation,
                            AnalyticsOperation::TopologyShareViewed
                                | AnalyticsOperation::TopologyEmbedViewed
                        )
                    {
                        continue;
                    }

                    let distinct_id = format!("org:{}", analytics_event.organization_id);
                    let event_name = analytics_event.operation.to_string();

                    let mut props = auth_properties(event);
                    props["organization_id"] = json!(analytics_event.organization_id.to_string());
                    if let Some(meta) = analytics_event.metadata.as_object() {
                        for (k, v) in meta {
                            props[k] = v.clone();
                        }
                    }

                    inject_org_group(&mut props);
                    self.capture(&event_name, &distinct_id, props).await;
                }
                Event::Discovery(discovery_event) => {
                    let event_name = match discovery_event.phase {
                        DiscoveryPhase::Pending => "discovery_started",
                        DiscoveryPhase::Complete => "discovery_completed",
                        DiscoveryPhase::Failed => "discovery_failed",
                        DiscoveryPhase::Cancelled => "discovery_cancelled",
                        _ => continue, // Filter should prevent this, but be safe
                    };

                    let Some(distinct_id) = self.resolve_distinct_id(event).await else {
                        tracing::debug!(
                            session_id = %discovery_event.session_id,
                            "Skipping PostHog discovery event — cannot attribute"
                        );
                        continue;
                    };

                    let mut props = auth_properties(event);
                    props["session_id"] = json!(discovery_event.session_id.to_string());
                    props["network_id"] = json!(discovery_event.network_id.to_string());
                    props["daemon_id"] = json!(discovery_event.daemon_id.to_string());

                    // Flatten discovery_type to a simple string instead of serialized JSON object
                    let type_name: &'static str = (&discovery_event.discovery_type).into();
                    props["discovery_type"] = json!(type_name);
                    if let DiscoveryType::Network { subnet_ids, .. } =
                        &discovery_event.discovery_type
                    {
                        props["discovery_subnet_scan"] = json!(subnet_ids.is_some());
                    }

                    // Include error_reason from metadata for failed discoveries
                    if let Some(error_reason) = discovery_event.metadata.get("error_reason") {
                        props["error_reason"] = error_reason.clone();
                    }

                    // Resolve org_id from network for discovery events
                    if let Some(org_id) = self
                        .get_org_id_from_network(&discovery_event.network_id)
                        .await
                    {
                        props["organization_id"] = json!(org_id.to_string());
                    }

                    inject_org_group(&mut props);
                    self.capture(event_name, &distinct_id, props).await;
                }
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "posthog"
    }

    fn debounce_window_ms(&self) -> u64 {
        5000
    }
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
        assert_eq!(to_snake_case("IfEntry"), "if_entry");
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
