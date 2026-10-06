use crate::server::{
    auth::middleware::auth::ActorProperties,
    shared::{
        events::{Attribution, Event, EventProperties, EventScope, Operation, ScopeOrganization},
        services::traits::CrudService,
    },
    sites::service::SiteService,
};
use backon::{ExponentialBuilder, Retryable};
use posthog_rs::ClientOptions;
use serde::Serialize;
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

pub struct PosthogService {
    client: posthog_rs::Client,
    site_service: Arc<SiteService>,
}

impl PosthogService {
    pub async fn new(api_key: String, api_host: String, site_service: Arc<SiteService>) -> Self {
        let options = ClientOptions::from((api_key.as_str(), api_host.as_str()));
        let client = posthog_rs::client(options).await;
        Self {
            client,
            site_service,
        }
    }

    /// Send a bus event under its own name and properties. Returns the distinct id it went to
    /// with the send's result, or `None` when it names neither a user nor an organization.
    pub async fn capture_event<Op: Operation>(
        &self,
        event: &Event<Op>,
    ) -> Option<(String, anyhow::Result<()>)> {
        let properties = self.event_properties(event).await;
        self.capture_as(&event.name(), event.attribution(), &properties)
            .await
    }

    /// [`Event::properties`], with a site-scoped event's organization looked up.
    pub async fn event_properties<'a, Op: Operation>(
        &self,
        event: &'a Event<Op>,
    ) -> EventProperties<impl Serialize + Send + 'a> {
        let site_organization_id = match event.scope.organization() {
            Some(ScopeOrganization::Site(site_id)) => self.get_org_id_from_site(&site_id).await,
            _ => None,
        };
        event.properties(site_organization_id)
    }

    /// Send `properties` as `name`, to the person `attribution` picks.
    pub async fn capture_as<M: Serialize>(
        &self,
        name: &str,
        attribution: Attribution,
        properties: &EventProperties<M>,
    ) -> Option<(String, anyhow::Result<()>)> {
        let Some(distinct_id) = distinct_id(attribution, &properties.actor) else {
            tracing::debug!(event = name, "Skipping PostHog event — cannot attribute");
            return None;
        };
        let result = self.capture(name, &distinct_id, properties).await;
        Some((distinct_id, result))
    }

    /// Send an event, retrying twice. Returns the last error once the retries run out.
    async fn capture<M: Serialize>(
        &self,
        name: &str,
        distinct_id: &str,
        properties: &EventProperties<M>,
    ) -> anyhow::Result<()> {
        let event = to_posthog_event(name, distinct_id, properties)
            .map_err(|e| anyhow::anyhow!("PostHog capture {name}: {e}"))?;

        (|| async { self.client.capture(event.clone()).await })
            .retry(
                ExponentialBuilder::default()
                    .with_min_delay(Duration::from_millis(100))
                    .with_max_delay(Duration::from_millis(500))
                    .with_max_times(2),
            )
            .await
            .map_err(|e| anyhow::anyhow!("PostHog capture {name}: {e}"))
    }

    /// Send a $identify event to set person properties in PostHog.
    pub async fn identify(
        &self,
        distinct_id: &str,
        properties: &impl Serialize,
    ) -> anyhow::Result<()> {
        let mut event = posthog_rs::Event::new("$identify", distinct_id);
        event
            .insert_prop("$set", properties)
            .map_err(|e| anyhow::anyhow!("PostHog $identify: {e}"))?;

        (|| async { self.client.capture(event.clone()).await })
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
        properties: &impl Serialize,
    ) -> anyhow::Result<()> {
        let context = || format!("PostHog $groupidentify {group_type}:{group_key}");
        let distinct_id = format!("group:{group_key}");
        let mut event = posthog_rs::Event::new("$groupidentify", &distinct_id);
        event
            .insert_prop("$group_type", group_type)
            .and_then(|()| event.insert_prop("$group_key", group_key))
            .and_then(|()| event.insert_prop("$group_set", properties))
            .map_err(|e| anyhow::anyhow!("{}: {e}", context()))?;

        (|| async { self.client.capture(event.clone()).await })
            .retry(
                ExponentialBuilder::default()
                    .with_min_delay(Duration::from_millis(100))
                    .with_max_delay(Duration::from_millis(500))
                    .with_max_times(2),
            )
            .await
            .map_err(|e| anyhow::anyhow!("{}: {e}", context()))
    }

    pub async fn get_org_id_from_site(&self, site_id: &Uuid) -> Option<Uuid> {
        if let Ok(Some(site)) = self.site_service.get_by_id(site_id).await {
            Some(site.base.organization_id)
        } else {
            None
        }
    }
}

/// The PostHog person an event lands on: a named user, else the actor, else the organization.
pub(crate) fn distinct_id(attribution: Attribution, actor: &ActorProperties) -> Option<String> {
    let organization = actor.organization_id.map(|id| format!("org:{id}"));
    match attribution {
        Attribution::User(user_id) => Some(user_id.to_string()),
        Attribution::Organization => organization,
        Attribution::Actor => actor.user_id.map(|id| id.to_string()).or(organization),
    }
}

/// The client-boundary conversion: properties serialize once, each top-level key becomes a
/// PostHog property, and the event joins its organization's group.
pub(crate) fn to_posthog_event<M: Serialize>(
    name: &str,
    distinct_id: &str,
    properties: &EventProperties<M>,
) -> anyhow::Result<posthog_rs::Event> {
    let mut event = posthog_rs::Event::new(name, distinct_id);
    let serde_json::Value::Object(props) = serde_json::to_value(properties)? else {
        anyhow::bail!("event properties did not serialize to an object");
    };
    for (key, value) in props {
        event.insert_prop(key, value)?;
    }
    if let Some(organization_id) = properties.actor.organization_id {
        event.add_group("organization", &organization_id.to_string());
    }
    Ok(event)
}
