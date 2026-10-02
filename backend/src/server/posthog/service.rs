use crate::server::{networks::service::NetworkService, shared::services::traits::CrudService};
use backon::{ExponentialBuilder, Retryable};
use posthog_rs::{ClientOptions, Event};
use std::{sync::Arc, time::Duration};
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

    /// Send an event, retrying twice. Returns the last error once the retries run out.
    pub async fn capture(
        &self,
        event_name: &str,
        distinct_id: &str,
        properties: serde_json::Value,
    ) -> anyhow::Result<()> {
        let event_name_owned = event_name.to_string();
        let distinct_id_owned = distinct_id.to_string();
        let props_clone = properties.clone();

        (|| async {
            let mut event = Event::new(&event_name_owned, &distinct_id_owned);
            if let Some(props) = props_clone.as_object() {
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

            self.client.capture(event).await
        })
        .retry(
            ExponentialBuilder::default()
                .with_min_delay(Duration::from_millis(100))
                .with_max_delay(Duration::from_millis(500))
                .with_max_times(2),
        )
        .await
        .map_err(|e| anyhow::anyhow!("PostHog capture {event_name_owned}: {e}"))
    }

    /// Send a $identify event to set person properties in PostHog.
    pub async fn identify(
        &self,
        distinct_id: &str,
        properties: serde_json::Value,
    ) -> anyhow::Result<()> {
        let distinct_id_owned = distinct_id.to_string();
        let props_clone = properties.clone();

        (|| async {
            let mut event = Event::new("$identify", &distinct_id_owned);
            event
                .insert_prop("$set", &props_clone)
                .map_err(|e| posthog_rs::Error::Connection(e.to_string()))?;
            self.client.capture(event).await
        })
        .retry(
            ExponentialBuilder::default()
                .with_min_delay(Duration::from_millis(100))
                .with_max_delay(Duration::from_millis(500))
                .with_max_times(2),
        )
        .await
        .map_err(|e| anyhow::anyhow!("PostHog $identify: {e}"))
    }

    /// Send a $groupidentify event to set group properties in PostHog.
    pub async fn group_identify(
        &self,
        group_type: &str,
        group_key: &str,
        properties: serde_json::Value,
    ) -> anyhow::Result<()> {
        let group_type_owned = group_type.to_string();
        let group_key_owned = group_key.to_string();
        let props_clone = properties.clone();

        (|| async {
            let distinct_id = format!("group:{}", group_key_owned);
            let mut event = Event::new("$groupidentify".to_string(), distinct_id);
            event
                .insert_prop("$group_type", &group_type_owned)
                .map_err(|e| posthog_rs::Error::Connection(e.to_string()))?;
            event
                .insert_prop("$group_key", &group_key_owned)
                .map_err(|e| posthog_rs::Error::Connection(e.to_string()))?;
            event
                .insert_prop("$group_set", &props_clone)
                .map_err(|e| posthog_rs::Error::Connection(e.to_string()))?;
            self.client.capture(event).await
        })
        .retry(
            ExponentialBuilder::default()
                .with_min_delay(Duration::from_millis(100))
                .with_max_delay(Duration::from_millis(500))
                .with_max_times(2),
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("PostHog $groupidentify {group_type_owned}:{group_key_owned}: {e}")
        })
    }

    pub async fn get_org_id_from_network(&self, network_id: &Uuid) -> Option<Uuid> {
        if let Ok(Some(network)) = self.network_service.get_by_id(network_id).await {
            Some(network.base.organization_id)
        } else {
            None
        }
    }
}
