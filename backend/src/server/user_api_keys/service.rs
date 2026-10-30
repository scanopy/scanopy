use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        api_key_common::ApiKeyService,
        entities::EntityDiscriminants,
        events::bus::EventBus,
        services::traits::{CrudService, EventBusService},
        storage::{
            child::{GenericChildStorage, ParentRef},
            generic::GenericPostgresStorage,
        },
    },
    tags::entity_tags::EntityTagService,
    user_api_keys::r#impl::{
        base::UserApiKey,
        site_access::{UserApiKeySiteAccess, UserApiKeySiteAccessBase},
    },
    users::r#impl::permissions::UserOrgPermissions,
};

pub struct UserApiKeyService {
    storage: Arc<GenericPostgresStorage<UserApiKey>>,
    site_access_storage: Arc<GenericChildStorage<UserApiKeySiteAccess>>,
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
        site_access_storage: Arc<GenericChildStorage<UserApiKeySiteAccess>>,
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

    /// Get a user API key by its hashed key value
    pub async fn get_by_key(&self, hashed_key: &str) -> Result<Option<UserApiKey>> {
        use crate::server::shared::storage::{filter::StorableFilter, traits::Storage};

        let filter = StorableFilter::<UserApiKey>::new_from_api_key(hashed_key.to_string());
        if let Some(mut key) = self.storage.get_unique(filter).await?.at_most_one()? {
            // Hydrate site_ids from junction table
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

    /// Fill each key's `site_ids` from the site-access junction.
    pub async fn hydrate_site_ids(&self, keys: &mut [UserApiKey]) -> Result<()> {
        let key_ids: Vec<Uuid> = keys.iter().map(|k| k.id).collect();
        let access_map = self.site_access_storage.get_for_parents(&key_ids).await?;
        for key in keys {
            key.base.site_ids = access_map
                .get(&key.id)
                .map(|access| access.iter().map(|a| a.site_id()).collect())
                .unwrap_or_default();
        }
        Ok(())
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
        let access = self.site_access_storage.get_for_parent(api_key_id).await?;
        Ok(access.iter().map(|a| a.site_id()).collect())
    }

    /// Create a new user API key with site access
    pub async fn create_with_sites(
        &self,
        api_key: UserApiKey,
        site_ids: Vec<Uuid>,
        authentication: AuthenticatedEntity,
    ) -> Result<UserApiKey> {
        // Store the key first: its site access rows reference it.
        let mut created = self.store_created(api_key, &authentication).await?;

        if !site_ids.is_empty() {
            self.update_site_access(&created.id, &site_ids).await?;
        }

        // Announced once its access is saved, with the site_ids hydrated, so a failed save
        // leaves no "created" event behind.
        created.base.site_ids = site_ids;
        self.publish_created(&created, authentication).await;
        Ok(created)
    }

    /// Update site access for an existing key
    pub async fn update_site_access(&self, api_key_id: &Uuid, site_ids: &[Uuid]) -> Result<()> {
        let rows: Vec<UserApiKeySiteAccess> = site_ids
            .iter()
            .map(|site_id| {
                UserApiKeySiteAccess::new(UserApiKeySiteAccessBase::new(*api_key_id, *site_id))
            })
            .collect();
        self.site_access_storage
            .save_for_parent(
                &ParentRef::new(EntityDiscriminants::UserApiKey, *api_key_id),
                &rows,
            )
            .await?;
        Ok(())
    }

    /// Give a key access to one more site, keeping the sites it already holds.
    pub async fn add_site_access(&self, api_key_id: &Uuid, site_id: &Uuid) -> Result<()> {
        let row = UserApiKeySiteAccess::new(UserApiKeySiteAccessBase::new(*api_key_id, *site_id));
        self.site_access_storage
            .add_for_parent(
                &ParentRef::new(EntityDiscriminants::UserApiKey, *api_key_id),
                &row,
            )
            .await?;
        Ok(())
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
