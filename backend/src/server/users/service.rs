use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        entities::ChangeTriggersTopologyStaleness,
        events::{
            bus::EventBus,
            types::{EntityEvent, EntityOperation},
        },
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Entity, Storable, Storage},
        },
    },
    tags::entity_tags::EntityTagService,
    users::r#impl::{
        base::User, network_access::UserNetworkAccessStorage, permissions::UserOrgPermissions,
    },
};
use anyhow::Error;
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

pub struct UserService {
    user_storage: Arc<GenericPostgresStorage<User>>,
    network_access_storage: Arc<UserNetworkAccessStorage>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<User> for UserService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_network_id(&self, _entity: &User) -> Option<Uuid> {
        None
    }
    fn get_organization_id(&self, entity: &User) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }
}

#[async_trait]
impl CrudService<User> for UserService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<User>> {
        &self.user_storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None // Users are not taggable entities
    }

    /// Create a new user
    async fn create(&self, user: User, authentication: AuthenticatedEntity) -> Result<User, Error> {
        let existing_user = self
            .user_storage
            .get_one(StorableFilter::<User>::new_from_email(&user.base.email))
            .await?;
        if existing_user.is_some() {
            return Err(anyhow::anyhow!(
                "User with email {} already exists",
                user.base.email
            ));
        }

        // Capture network_ids before creating the user (since they're stored in junction table)
        let network_ids = user.base.network_ids.clone();

        let user = if user.id() == Uuid::nil() {
            User::new(user.base)
        } else {
            user
        };
        let created = self.user_storage.create(&user).await?;

        // Persist network_ids to the junction table
        if !network_ids.is_empty() {
            self.set_network_ids(&created.id, &network_ids).await?;
        }

        let trigger_stale = created.triggers_staleness(None);

        let metadata = serde_json::json!({
            "trigger_stale": trigger_stale
        });

        self.event_bus()
            .publish_entity(EntityEvent {
                id: Uuid::new_v4(),
                entity_id: created.id,
                network_id: self.get_network_id(&created),
                organization_id: self.get_organization_id(&created),
                entity_type: created.clone().into(),
                operation: EntityOperation::Created,
                timestamp: Utc::now(),
                metadata,

                authentication,
            })
            .await?;

        Ok(created)
    }
}

impl UserService {
    pub fn new(
        user_storage: Arc<GenericPostgresStorage<User>>,
        network_access_storage: Arc<UserNetworkAccessStorage>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            user_storage,
            network_access_storage,
            event_bus,
        }
    }

    pub async fn get_user_by_oidc(&self, oidc_subject: &str) -> Result<Option<User>> {
        let oidc_filter = StorableFilter::<User>::new_from_oidc_subject(oidc_subject.to_string());
        self.user_storage.get_one(oidc_filter).await
    }

    pub async fn get_organization_owners(&self, organization_id: &Uuid) -> Result<Vec<User>> {
        let filter = StorableFilter::<User>::new_from_org_id(organization_id)
            .user_permissions(&UserOrgPermissions::Owner);

        self.user_storage.get_all(filter).await
    }

    /// Get network_ids for a user from the user_network_access junction table
    pub async fn get_network_ids(&self, user_id: &Uuid) -> Result<Vec<Uuid>> {
        self.network_access_storage.get_for_user(user_id).await
    }

    /// Set network_ids for a user - replaces all existing entries in user_network_access
    pub async fn set_network_ids(&self, user_id: &Uuid, network_ids: &[Uuid]) -> Result<()> {
        self.network_access_storage
            .save_for_user(user_id, network_ids)
            .await
    }

    /// Add a network_id to a user's access
    pub async fn add_network_access(&self, user_id: &Uuid, network_id: &Uuid) -> Result<()> {
        self.network_access_storage
            .add_network(user_id, network_id)
            .await
    }

    /// Remove a network_id from a user's access
    pub async fn remove_network_access(&self, user_id: &Uuid, network_id: &Uuid) -> Result<()> {
        self.network_access_storage
            .remove_network(user_id, network_id)
            .await
    }

    /// Hydrate network_ids for a single user
    pub async fn hydrate_network_ids(&self, user: &mut User) -> Result<()> {
        user.base.network_ids = self.network_access_storage.get_for_user(&user.id).await?;
        Ok(())
    }

    /// Hydrate network_ids for multiple users (batch operation)
    pub async fn hydrate_network_ids_batch(&self, users: &mut [User]) -> Result<()> {
        if users.is_empty() {
            return Ok(());
        }

        let user_ids: Vec<Uuid> = users.iter().map(|u| u.id).collect();
        let mut network_map = self.network_access_storage.get_for_users(&user_ids).await?;

        for user in users.iter_mut() {
            user.base.network_ids = network_map.remove(&user.id).unwrap_or_default();
        }

        Ok(())
    }
}
