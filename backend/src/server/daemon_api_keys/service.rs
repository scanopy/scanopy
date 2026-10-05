use anyhow::{Result, anyhow};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    daemon_api_keys::r#impl::base::DaemonApiKey,
    shared::{
        api_key_common::ApiKeyService,
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService},
        storage::generic::GenericPostgresStorage,
    },
    tags::entity_tags::EntityTagService,
};

/// The minimum a daemon request needs to authenticate + resolve its identity,
/// cached by hashed key so hot poll loops don't hit the DB every ~30s. Validity
/// (`is_enabled`/`expires_at`) is re-evaluated on each cache hit, not cached as a
/// verdict; a short TTL backstops mutations that don't explicitly evict.
#[derive(Clone)]
pub struct ResolvedDaemonKey {
    pub api_key_id: Uuid,
    pub site_id: Uuid,
    /// The daemon bound 1:1 to this key, or None for a legacy site-shared key.
    pub daemon_id: Option<Uuid>,
    pub is_enabled: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Short enough that an un-evicted disable/expiry propagates quickly, long enough
/// to absorb a fleet of 30s poll loops.
const RESOLUTION_CACHE_TTL: Duration = Duration::from_secs(10);

pub struct DaemonApiKeyService {
    storage: Arc<GenericPostgresStorage<DaemonApiKey>>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
    /// hashed_key -> resolution. Populated on auth cache-miss, evicted on
    /// rotate/delete/update of the key (see the daemon_api_keys handlers).
    resolution_cache: Cache<String, ResolvedDaemonKey>,
}

impl EventBusService<DaemonApiKey> for DaemonApiKeyService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, entity: &DaemonApiKey) -> Option<Uuid> {
        Some(entity.base.site_id)
    }

    fn get_organization_id(&self, _entity: &DaemonApiKey) -> Option<Uuid> {
        None
    }

    fn suppress_logs(
        &self,
        current: Option<&DaemonApiKey>,
        updated: Option<&DaemonApiKey>,
    ) -> bool {
        match (current, updated) {
            (Some(current), Some(updated)) => updated.suppress_logs(current),
            _ => false,
        }
    }
}

#[async_trait]
impl CrudService<DaemonApiKey> for DaemonApiKeyService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<DaemonApiKey>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }
}

impl DaemonApiKeyService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<DaemonApiKey>>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            storage,
            event_bus,
            entity_tag_service,
            resolution_cache: Cache::builder().time_to_live(RESOLUTION_CACHE_TTL).build(),
        }
    }

    /// Read a cached auth resolution for a hashed key, if present. The caller
    /// re-checks validity (`is_enabled`/`expires_at`) on the returned value.
    pub async fn cached_resolution(&self, hashed_key: &str) -> Option<ResolvedDaemonKey> {
        self.resolution_cache.get(hashed_key).await
    }

    /// Populate the resolution cache after a fresh DB load in the auth path.
    pub async fn cache_resolution(&self, hashed_key: &str, resolved: ResolvedDaemonKey) {
        self.resolution_cache
            .insert(hashed_key.to_string(), resolved)
            .await;
    }

    /// Evict a hashed key from the resolution cache. Call on rotate (old hash),
    /// delete, or any update that can change validity/binding, so the change
    /// takes effect without waiting out the TTL.
    pub async fn invalidate_resolution(&self, hashed_key: &str) {
        self.resolution_cache.invalidate(hashed_key).await;
    }
}

impl ApiKeyService for DaemonApiKeyService {
    type Key = DaemonApiKey;

    fn api_key_event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn validate_access(&self, key: &DaemonApiKey, entity: &AuthenticatedEntity) -> Result<()> {
        // User must have access to the site this key belongs to
        if !entity.site_ids().contains(&key.base.site_id) {
            return Err(anyhow!(
                "You don't have access to the site for this daemon API key"
            ));
        }
        Ok(())
    }
}
