use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        entities::ChangeTriggersTopologyStaleness,
        events::{bus::EventBus, types::EntityOperation},
        handlers::ordering::OrderField,
        services::search::{SearchQuery, SearchScope},
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Entity, Storable, Storage},
        },
    },
    tags::entity_tags::EntityTagService,
    users::r#impl::{
        base::User, permissions::UserOrgPermissions, site_access::UserSiteAccessStorage,
    },
};
use anyhow::Error;
use anyhow::Result;
use async_trait::async_trait;
use email_address::EmailAddress;
use std::sync::Arc;
use uuid::Uuid;

pub struct UserService {
    user_storage: Arc<GenericPostgresStorage<User>>,
    site_access_storage: Arc<UserSiteAccessStorage>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<User> for UserService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, _entity: &User) -> Option<Uuid> {
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

    /// The users list's rule: admins and owners only, and only the users that list shows them.
    /// Filtered after the query, as the list is, so the limit applies to what the caller sees.
    async fn search<O: OrderField>(
        &self,
        scope: &SearchScope,
        query: &SearchQuery,
        _order_by: Option<O>,
    ) -> Result<Vec<User>, Error> {
        if scope.permissions < UserOrgPermissions::Admin {
            return Ok(Vec::new());
        }
        let Some(filter) = query.narrow(StorableFilter::<User>::new_from_org_id(
            &scope.organization_id,
        )) else {
            return Ok(Vec::new());
        };
        Ok(self
            .user_storage
            .get_all_ordered(filter, "users.email ASC")
            .await?
            .into_iter()
            .filter(|user| user.is_listed_for(scope.permissions, scope.user_id))
            .take(query.limit as usize)
            .collect())
    }

    /// Create a new user
    async fn create(&self, user: User, authentication: AuthenticatedEntity) -> Result<User, Error> {
        let email_taken = self
            .user_storage
            .exists(StorableFilter::<User>::new_from_email(&user.base.email))
            .await?;
        if email_taken {
            return Err(anyhow::anyhow!(
                "User with email {} already exists",
                user.base.email
            ));
        }

        // Capture site_ids before creating the user (since they're stored in junction table)
        let site_ids = user.base.site_ids.clone();

        let user = if user.id() == Uuid::nil() {
            User::new(user.base)
        } else {
            user
        };
        let created = self.user_storage.create(&user).await?;

        // Persist site_ids to the junction table
        if !site_ids.is_empty() {
            self.set_site_ids(&created.id, &site_ids).await?;
        }

        let trigger_stale = created.triggers_staleness(None);

        if let Some(scope) = EntityScope::from_ids(
            created.id,
            created.clone().into(),
            self.get_site_id(&created),
            self.get_organization_id(&created),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Created, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        Ok(created)
    }
}

impl UserService {
    pub fn new(
        user_storage: Arc<GenericPostgresStorage<User>>,
        site_access_storage: Arc<UserSiteAccessStorage>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            user_storage,
            site_access_storage,
            event_bus,
        }
    }

    pub async fn get_user_by_oidc(&self, oidc_subject: &str) -> Result<Option<User>> {
        let oidc_filter = StorableFilter::<User>::new_from_oidc_subject(oidc_subject.to_string());
        self.user_storage
            .get_unique(oidc_filter)
            .await?
            .at_most_one()
    }

    /// Look up a user by their email address. Used by the email send pipeline
    /// to resolve the recipient's pause preferences.
    pub async fn get_by_email(&self, email: &EmailAddress) -> Result<Option<User>> {
        self.user_storage
            .get_unique(StorableFilter::<User>::new_from_email(email))
            .await?
            .at_most_one()
    }

    pub async fn get_organization_owners(&self, organization_id: &Uuid) -> Result<Vec<User>> {
        let filter = StorableFilter::<User>::new_from_org_id(organization_id)
            .user_permissions(&UserOrgPermissions::Owner);

        self.user_storage.get_all(filter).await
    }

    /// Returns all users with access to `site_id` within `organization_id`.
    ///
    /// Access is the union of:
    /// - Users with an explicit row in the `user_site_access` junction.
    /// - Org Owners and Admins, who have implicit access to every site in
    ///   their organization regardless of the junction table.
    ///
    /// Deduplicates users that fall into both sets. The org-id parameter is
    /// required so the implicit set is scoped — junction rows alone don't
    /// carry the org context the way `User.organization_id` does.
    pub async fn get_users_with_site_access(
        &self,
        site_id: &Uuid,
        organization_id: &Uuid,
    ) -> Result<Vec<User>> {
        let explicit_user_ids = self
            .site_access_storage
            .get_user_ids_for_site(site_id)
            .await?;

        let explicit = if explicit_user_ids.is_empty() {
            Vec::new()
        } else {
            // The users table's PK is `id`; the `user_id` column lives on the
            // user_site_access junction. Filter on entity ids.
            let filter = StorableFilter::<User>::new_from_entity_ids(&explicit_user_ids);
            self.user_storage.get_all(filter).await?
        };

        let implicit_filter = StorableFilter::<User>::new_from_org_id(organization_id)
            .user_permissions_in(&[UserOrgPermissions::Owner, UserOrgPermissions::Admin]);
        let implicit = self.user_storage.get_all(implicit_filter).await?;

        let mut seen: std::collections::HashSet<Uuid> =
            std::collections::HashSet::with_capacity(explicit.len() + implicit.len());
        let mut out: Vec<User> = Vec::with_capacity(explicit.len() + implicit.len());
        for user in explicit.into_iter().chain(implicit.into_iter()) {
            if seen.insert(user.id) {
                out.push(user);
            }
        }
        Ok(out)
    }

    /// Get site_ids for a user from the user_site_access junction table
    pub async fn get_site_ids(&self, user_id: &Uuid) -> Result<Vec<Uuid>> {
        self.site_access_storage.get_for_user(user_id).await
    }

    /// Set site_ids for a user - replaces all existing entries in user_site_access
    pub async fn set_site_ids(&self, user_id: &Uuid, site_ids: &[Uuid]) -> Result<()> {
        self.site_access_storage
            .save_for_user(user_id, site_ids)
            .await
    }

    /// Add a site_id to a user's access
    pub async fn add_site_access(&self, user_id: &Uuid, site_id: &Uuid) -> Result<()> {
        self.site_access_storage.add_site(user_id, site_id).await
    }

    /// Remove a site_id from a user's access
    pub async fn remove_site_access(&self, user_id: &Uuid, site_id: &Uuid) -> Result<()> {
        self.site_access_storage.remove_site(user_id, site_id).await
    }

    /// Hydrate site_ids for a single user
    pub async fn hydrate_site_ids(&self, user: &mut User) -> Result<()> {
        user.base.site_ids = self.site_access_storage.get_for_user(&user.id).await?;
        Ok(())
    }

    /// Hydrate site_ids for multiple users (batch operation)
    pub async fn hydrate_site_ids_batch(&self, users: &mut [User]) -> Result<()> {
        if users.is_empty() {
            return Ok(());
        }

        let user_ids: Vec<Uuid> = users.iter().map(|u| u.id).collect();
        let mut site_map = self.site_access_storage.get_for_users(&user_ids).await?;

        for user in users.iter_mut() {
            user.base.site_ids = site_map.remove(&user.id).unwrap_or_default();
        }

        Ok(())
    }
}
