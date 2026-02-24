use crate::{
    daemon::discovery::types::base::DiscoveryPhase,
    server::{
        posthog::service::PosthogService,
        shared::{
            entities::EntityDiscriminants,
            events::{
                bus::{EventFilter, EventSubscriber},
                types::{AuthOperation, EntityOperation, Event, TelemetryOperation},
            },
        },
    },
};
use anyhow::Error;
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;

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
        entity_ops.insert(EntityDiscriminants::SnmpCredential, ops.clone());
        entity_ops.insert(EntityDiscriminants::Invite, ops.clone());
        entity_ops.insert(EntityDiscriminants::User, ops);

        EventFilter {
            entity_operations: Some(entity_ops),
            auth_operations: Some(vec![AuthOperation::LoginSuccess]),
            telemetry_operations: Some(vec![
                // Billing lifecycle
                TelemetryOperation::CheckoutStarted,
                TelemetryOperation::CheckoutCompleted,
                TelemetryOperation::TrialStarted,
                TelemetryOperation::TrialEnded,
                TelemetryOperation::TrialWillEnd,
                TelemetryOperation::SubscriptionCancelled,
                TelemetryOperation::PlanChanged,
                TelemetryOperation::PaymentFailed,
                TelemetryOperation::PaymentActionRequired,
                TelemetryOperation::PaymentRecovered,
                // Onboarding & activation milestones
                TelemetryOperation::OrgCreated,
                TelemetryOperation::OnboardingModalCompleted,
                TelemetryOperation::PlanSelected,
                // Engagement milestones (fire once per org)
                TelemetryOperation::FirstDaemonRegistered,
                TelemetryOperation::FirstTopologyRebuild,
                TelemetryOperation::FirstDiscoveryCompleted,
                TelemetryOperation::FirstHostDiscovered,
                TelemetryOperation::SecondNetworkCreated,
                TelemetryOperation::FirstTagCreated,
                TelemetryOperation::FirstGroupCreated,
                TelemetryOperation::FirstUserApiKeyCreated,
                TelemetryOperation::FirstSnmpCredentialCreated,
                TelemetryOperation::InviteSent,
                TelemetryOperation::InviteAccepted,
            ]),
            discovery_phases: Some(vec![
                DiscoveryPhase::Pending,
                DiscoveryPhase::Complete,
                DiscoveryPhase::Failed,
                DiscoveryPhase::Cancelled,
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

                    self.capture("login", &distinct_id, props).await;
                }
                Event::Telemetry(telemetry_event) => {
                    let Some(distinct_id) = self.resolve_distinct_id(event).await else {
                        tracing::debug!(
                            operation = %telemetry_event.operation,
                            "Skipping PostHog telemetry event — cannot attribute"
                        );
                        continue;
                    };

                    let event_name = telemetry_event.operation.to_string();

                    let mut props = auth_properties(event);
                    props["organization_id"] = json!(telemetry_event.organization_id.to_string());
                    props["metadata"] = telemetry_event.metadata.clone();

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
                    props["discovery_type"] = serde_json::to_value(&discovery_event.discovery_type)
                        .unwrap_or(json!(null));

                    // Resolve org_id from network for discovery events
                    if let Some(org_id) = self
                        .get_org_id_from_network(&discovery_event.network_id)
                        .await
                    {
                        props["organization_id"] = json!(org_id.to_string());
                    }

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
        assert_eq!(to_snake_case("SnmpCredential"), "snmp_credential");
        assert_eq!(to_snake_case("Network"), "network");
        assert_eq!(to_snake_case("IfEntry"), "if_entry");
    }
}
