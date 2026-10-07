use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        api_key_common::ApiKeyService,
        events::bus::EventBus,
        handlers::ordering::OrderField,
        services::search::{SearchQuery, SearchScope},
        services::traits::{CrudService, EventBusService},
        storage::{filter::StorableFilter, generic::GenericPostgresStorage, traits::Storage},
    },
    tags::entity_tags::EntityTagService,
    user_api_keys::r#impl::{base::UserApiKey, site_access::UserApiKeySiteAccessStorage},
    users::r#impl::permissions::UserOrgPermissions,
};

pub struct UserApiKeyService {
    storage: Arc<GenericPostgresStorage<UserApiKey>>,
    site_access_storage: Arc<UserApiKeySiteAccessStorage>,
    event_bus: Arc<EventBus>,
    entity_tag_service: Arc<EntityTagService>,
}

impl EventBusService<UserApiKey> for UserApiKeyService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, _entity: &UserApiKey) -> Option<Uuid> {
        // User API keys use junction table, not a single site_id
        None
    }

    fn get_organization_id(&self, entity: &UserApiKey) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }

    fn suppress_logs(&self, current: Option<&UserApiKey>, updated: Option<&UserApiKey>) -> bool {
        match (current, updated) {
            (Some(current), Some(updated)) => updated.suppress_logs(current),
            _ => false,
        }
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

    /// Only the signed-in user's own keys, as their list shows them. An API key caller lists
    /// none, so it finds none.
    async fn search<O: OrderField>(
        &self,
        scope: &SearchScope,
        query: &SearchQuery,
        _order_by: Option<O>,
    ) -> Result<Vec<UserApiKey>> {
        let Some(user_id) = scope.session_user_id else {
            return Ok(Vec::new());
        };
        let Some(filter) = query.narrow(StorableFilter::<UserApiKey>::new_from_user_id(&user_id))
        else {
            return Ok(Vec::new());
        };
        let keys = self
            .storage
            .get_all_ordered(filter.limit(query.limit), "user_api_keys.name ASC")
            .await?;
        self.hydrate(keys).await
    }
}

impl UserApiKeyService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<UserApiKey>>,
        site_access_storage: Arc<UserApiKeySiteAccessStorage>,
        event_bus: Arc<EventBus>,
        entity_tag_service: Arc<EntityTagService>,
    ) -> Self {
        Self {
            storage,
            site_access_storage,
            event_bus,
            entity_tag_service,
        }
    }

    /// Get the site access storage for junction table operations
    pub fn site_access_storage(&self) -> &Arc<UserApiKeySiteAccessStorage> {
        &self.site_access_storage
    }

    /// Get a user API key by its hashed key value
    pub async fn get_by_key(&self, hashed_key: &str) -> Result<Option<UserApiKey>> {
        let filter = StorableFilter::<UserApiKey>::new_from_api_key(hashed_key.to_string());
        if let Some(mut key) = self.storage.get_unique(filter).await?.at_most_one()? {
            // Hydrate site_ids from junction table
            key.base.site_ids = self.site_access_storage.get_for_key(&key.id).await?;
            self.hydrate_tags(&mut key).await?;
            return Ok(Some(key));
        }
        Ok(None)
    }

    /// Get all API keys for a specific user, with site_ids hydrated
    pub async fn get_for_user(&self, user_id: &Uuid) -> Result<Vec<UserApiKey>> {
        use crate::server::shared::storage::{filter::StorableFilter, traits::Storage};

        let filter = StorableFilter::<UserApiKey>::new_from_user_id(user_id);
        let keys = self.storage.get_all(filter).await?;
        self.hydrate(keys).await
    }

    /// Fill in the site access and tags `storage` reads leave empty.
    async fn hydrate(&self, mut keys: Vec<UserApiKey>) -> Result<Vec<UserApiKey>> {
        // Batch hydrate site_ids
        let key_ids: Vec<Uuid> = keys.iter().map(|k| k.id).collect();
        let site_map = self.site_access_storage.get_for_keys(&key_ids).await?;

        for key in &mut keys {
            key.base.site_ids = site_map.get(&key.id).cloned().unwrap_or_default();
        }

        self.bulk_hydrate_tags(&mut keys, None).await?;

        Ok(keys)
    }

    /// Validate that the requested permissions don't exceed the user's permissions
    pub fn validate_permissions(
        key_permissions: UserOrgPermissions,
        user_permissions: UserOrgPermissions,
    ) -> Result<(), String> {
        if key_permissions > user_permissions {
            return Err(format!(
                "API key permissions ({}) cannot exceed your permissions ({})",
                key_permissions, user_permissions
            ));
        }
        Ok(())
    }

    /// Get site IDs for an API key from the junction table
    pub async fn get_site_ids(&self, api_key_id: &Uuid) -> Result<Vec<Uuid>> {
        self.site_access_storage.get_for_key(api_key_id).await
    }

    /// Create a new user API key with site access
    pub async fn create_with_sites(
        &self,
        api_key: UserApiKey,
        site_ids: Vec<Uuid>,
        authentication: AuthenticatedEntity,
    ) -> Result<UserApiKey> {
        // Create the key first
        let created = self.create(api_key.clone(), authentication).await?;

        // Then save site access
        if !site_ids.is_empty() {
            self.site_access_storage
                .save_for_key(&created.id, &site_ids)
                .await?;
        }

        // Return with hydrated site_ids
        let mut result = created;
        result.base.site_ids = site_ids;
        Ok(result)
    }

    /// Update site access for an existing key
    pub async fn update_site_access(&self, api_key_id: &Uuid, site_ids: &[Uuid]) -> Result<()> {
        self.site_access_storage
            .save_for_key(api_key_id, site_ids)
            .await
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
