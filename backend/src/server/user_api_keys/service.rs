use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        api_key_common::ApiKeyService,
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService},
        storage::generic::GenericPostgresStorage,
    },
    site_assignments::{r#impl::base::Principal, service::SiteAssignmentService},
    tags::entity_tags::EntityTagService,
    user_api_keys::r#impl::base::UserApiKey,
};

pub struct UserApiKeyService {
    storage: Arc<GenericPostgresStorage<UserApiKey>>,
    site_assignments: Arc<SiteAssignmentService>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
}

impl EventBusService<UserApiKey> for UserApiKeyService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, _entity: &UserApiKey) -> Option<Uuid> {
        // A key's sites are its site assignments, not a single site_id
        None
    }

    fn get_organization_id(&self, entity: &UserApiKey) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }
}

#[async_trait]
impl CrudService<UserApiKey> for UserApiKeyService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<UserApiKey>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        Some(&self.entity_tag_service)
    }
}

impl UserApiKeyService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<UserApiKey>>,
        site_assignments: Arc<SiteAssignmentService>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            storage,
            site_assignments,
            event_bus,
            entity_tag_service,
        }
    }

    /// Get a user API key by its hashed key value
    pub async fn get_by_key(&self, hashed_key: &str) -> Result<Option<UserApiKey>> {
        use crate::server::shared::storage::{filter::StorableFilter, traits::Storage};

        let filter = StorableFilter::<UserApiKey>::new_from_api_key(hashed_key.to_string());
        if let Some(mut key) = self.storage.get_unique(filter).await?.at_most_one()? {
            key.base.site_ids = self.get_site_ids(&key.id).await?;
            self.hydrate_tags(&mut key).await?;
            return Ok(Some(key));
        }
        Ok(None)
    }

    /// Get all API keys for a specific user, with site_ids hydrated
    pub async fn get_for_user(&self, user_id: &Uuid) -> Result<Vec<UserApiKey>> {
        use crate::server::shared::storage::{filter::StorableFilter, traits::Storage};

        let filter = StorableFilter::<UserApiKey>::new_from_user_id(user_id);
        let mut keys = self.storage.get_all(filter).await?;
        self.hydrate_site_ids(&mut keys).await?;
        self.bulk_hydrate_tags(&mut keys, None).await?;

        Ok(keys)
    }

    /// Fill each key's `site_ids` from its site assignments: `None` for All sites, and an empty
    /// list for a key from before site assignments, which has no rows.
    pub async fn hydrate_site_ids(&self, keys: &mut [UserApiKey]) -> Result<()> {
        let key_ids: Vec<Uuid> = keys.iter().map(|k| k.id).collect();
        let mut scopes = self
            .site_assignments
            .site_scopes_by("api_key_id", &key_ids, |principal| match principal {
                Principal::ApiKey(id) => Some(*id),
                _ => None,
            })
            .await?;
        for key in keys {
            key.base.site_ids = scopes.remove(&key.id).unwrap_or_else(|| Some(Vec::new()));
        }
        Ok(())
    }

    /// An API key's `site_ids`, as [`Self::hydrate_site_ids`] fills them.
    pub async fn get_site_ids(&self, api_key_id: &Uuid) -> Result<Option<Vec<Uuid>>> {
        let mut scopes = self
            .site_assignments
            .site_scopes_by("api_key_id", &[*api_key_id], |principal| match principal {
                Principal::ApiKey(id) => Some(*id),
                _ => None,
            })
            .await?;
        Ok(scopes
            .remove(api_key_id)
            .unwrap_or_else(|| Some(Vec::new())))
    }
}

impl ApiKeyService for UserApiKeyService {
    type Key = UserApiKey;

    fn api_key_event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn validate_access(&self, key: &UserApiKey, entity: &AuthenticatedEntity) -> Result<()> {
        // User must own this key
        if let Some(user_id) = entity.user_id() {
            if key.base.user_id != user_id {
                return Err(anyhow!("You don't own this API key"));
            }
            Ok(())
        } else {
            Err(anyhow!("User context required to validate API key access"))
        }
    }
}
