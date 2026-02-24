use crate::server::{networks::service::NetworkService, shared::services::traits::CrudService};
use posthog_rs::{ClientOptions, Event};
use std::sync::Arc;
use uuid::Uuid;

pub struct PosthogService {
    client: posthog_rs::Client,
    network_service: Arc<NetworkService>,
}

impl PosthogService {
    pub async fn new(
        api_key: String,
        api_host: String,
        network_service: Arc<NetworkService>,
    ) -> Self {
        let options = ClientOptions::from((api_key.as_str(), api_host.as_str()));
        let client = posthog_rs::client(options).await;
        Self {
            client,
            network_service,
        }
    }

    pub async fn capture(
        &self,
        event_name: &str,
        distinct_id: &str,
        properties: serde_json::Value,
    ) {
        let mut event = Event::new(event_name, distinct_id);
        if let Some(props) = properties.as_object() {
            for (key, value) in props {
                // Handle $groups specially via add_group() for proper PostHog group analytics
                if key == "$groups" {
                    if let Some(groups) = value.as_object() {
                        for (group_type, group_key) in groups {
                            if let Some(key_str) = group_key.as_str() {
                                event.add_group(group_type, key_str);
                            }
                        }
                    }
                    continue;
                }
                // insert_prop requires a Serialize type; serde_json::Value implements it
                if let Err(e) = event.insert_prop(key, value) {
                    tracing::warn!(key = %key, error = %e, "Failed to insert PostHog event property");
                }
            }
        }

        if let Err(e) = self.client.capture(event).await {
            tracing::warn!(event = %event_name, error = %e, "Failed to send event to PostHog");
        }
    }

    /// Send a $identify event to set person properties in PostHog.
    pub async fn identify(&self, distinct_id: &str, properties: serde_json::Value) {
        let mut event = Event::new("$identify", distinct_id);
        if let Err(e) = event.insert_prop("$set", &properties) {
            tracing::warn!(error = %e, "Failed to insert $set property for PostHog identify");
            return;
        }
        if let Err(e) = self.client.capture(event).await {
            tracing::warn!(error = %e, "Failed to send $identify event to PostHog");
        }
    }

    /// Send a $groupidentify event to set group properties in PostHog.
    pub async fn group_identify(
        &self,
        group_type: &str,
        group_key: &str,
        properties: serde_json::Value,
    ) {
        let distinct_id = format!("group:{group_key}");
        let mut event = Event::new("$groupidentify".to_string(), distinct_id);
        if let Err(e) = event.insert_prop("$group_type", group_type) {
            tracing::warn!(error = %e, "Failed to insert $group_type for PostHog group_identify");
            return;
        }
        if let Err(e) = event.insert_prop("$group_key", group_key) {
            tracing::warn!(error = %e, "Failed to insert $group_key for PostHog group_identify");
            return;
        }
        if let Err(e) = event.insert_prop("$group_set", &properties) {
            tracing::warn!(error = %e, "Failed to insert $group_set for PostHog group_identify");
            return;
        }
        if let Err(e) = self.client.capture(event).await {
            tracing::warn!(error = %e, "Failed to send $groupidentify event to PostHog");
        }
    }

    pub async fn get_org_id_from_network(&self, network_id: &Uuid) -> Option<Uuid> {
        if let Ok(Some(network)) = self.network_service.get_by_id(network_id).await {
            Some(network.base.organization_id)
        } else {
            None
        }
    }
}
