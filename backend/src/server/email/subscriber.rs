use std::collections::HashMap;

use anyhow::Error;
use async_trait::async_trait;

use crate::{
    daemon::discovery::types::base::DiscoveryPhase,
    server::{
        email::traits::EmailService,
        shared::{
            entities::EntityDiscriminants,
            events::{
                bus::{EventFilter, EventSubscriber},
                types::{EntityOperation, Event, OnboardingOperation},
            },
            services::traits::CrudService,
        },
    },
};

#[async_trait]
impl EventSubscriber for EmailService {
    fn event_filter(&self) -> EventFilter {
        EventFilter {
            entity_operations: Some(HashMap::from([
                (
                    EntityDiscriminants::Host,
                    Some(vec![EntityOperation::Created, EntityOperation::Deleted]),
                ),
                (
                    EntityDiscriminants::Network,
                    Some(vec![EntityOperation::Created, EntityOperation::Deleted]),
                ),
                (
                    EntityDiscriminants::User,
                    Some(vec![EntityOperation::Created, EntityOperation::Deleted]),
                ),
            ])),
            billing_operations: Some(vec![]),
            onboarding_operations: Some(vec![
                OnboardingOperation::FirstDaemonRegistered,
                OnboardingOperation::FirstDiscoveryCompleted,
            ]),
            auth_operations: Some(vec![]),
            discovery_phases: Some(vec![DiscoveryPhase::Failed]),
            analytics_operations: Some(vec![]),
            network_ids: None,
        }
    }

    async fn handle_events(&self, events: Vec<Event>) -> Result<(), Error> {
        if events.is_empty() {
            return Ok(());
        }

        for event in events {
            match event {
                Event::Entity(e) => {
                    tracing::debug!(
                        entity_type = ?e.entity_type,
                        operation = ?e.operation,
                        entity_id = %e.entity_id,
                        "Email subscriber received entity event"
                    );

                    let org_id = if let Some(org_id) = e.organization_id {
                        Some(org_id)
                    } else if let Some(network_id) = e.network_id {
                        self.network_service
                            .get_by_id(&network_id)
                            .await?
                            .map(|n| n.base.organization_id)
                    } else {
                        None
                    };

                    if let Some(org_id) = org_id
                        && let Err(e) = self
                            .check_plan_limits(org_id, e.operation == EntityOperation::Deleted)
                            .await
                    {
                        tracing::warn!(
                            organization_id = %org_id,
                            error = %e,
                            "Failed to check plan limits"
                        );
                    }
                }
                Event::Onboarding(t) => {
                    tracing::debug!(
                        operation = ?t.operation,
                        organization_id = %t.organization_id,
                        "Email subscriber received onboarding event"
                    );
                    match t.operation {
                        OnboardingOperation::FirstDaemonRegistered => {
                            let daemon_name = t
                                .metadata
                                .get("daemon_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("your daemon");
                            let network_name = t
                                .metadata
                                .get("network_name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("your network");

                            if let Err(e) = self
                                .send_discovery_guide_for_org(
                                    t.organization_id,
                                    daemon_name,
                                    network_name,
                                )
                                .await
                            {
                                tracing::warn!(
                                    organization_id = %t.organization_id,
                                    error = %e,
                                    "Failed to send discovery guide email"
                                );
                            }
                        }
                        OnboardingOperation::FirstDiscoveryCompleted => {
                            // Only send topology ready email for Network/Unified discoveries, not SelfReport
                            let is_network = t
                                .metadata
                                .get("discovery_type")
                                .and_then(|v| v.as_str())
                                .map(|dt| dt.starts_with("Network") || dt.starts_with("Unified"))
                                .unwrap_or(false);

                            if is_network
                                && let Err(e) =
                                    self.send_topology_ready_for_org(t.organization_id).await
                            {
                                tracing::warn!(
                                    organization_id = %t.organization_id,
                                    error = %e,
                                    "Failed to send topology ready email"
                                );
                            }
                        }
                        _ => {}
                    }
                }
                Event::Discovery(_) => {}
                _ => {}
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "email_onboarding_and_plan_limits"
    }
}
