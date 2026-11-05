use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        entities::ChangeTriggersTopologyStaleness,
        events::{
            bus::EventBus,
            types::{EntityOperation, UpdateReason},
        },
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            lock::{DEFAULT_LOCK_TIMEOUT, LockKey, SessionLockGuard, xact_lock},
            traits::{Entity, Storable, Storage},
        },
    },
    site_assignments::{r#impl::base::Principal, service::SiteAssignmentService},
    tags::entity_tags::EntityTagService,
    users::r#impl::{base::User, email_settings::EmailSettings, permissions::UserOrgPermissions},
};
use anyhow::Error;
use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDate;
use email_address::EmailAddress;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use uuid::Uuid;

pub struct UserService {
    user_storage: Arc<GenericPostgresStorage<User>>,
    site_assignments: Arc<SiteAssignmentService>,
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

    /// Create a new user. `AccessService::set_access` gives them their sites.
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

        let user = if user.id() == Uuid::nil() {
            User::new(user.base)
        } else {
            user
        };
        let created = self.user_storage.create(&user).await?;

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
                .await;
        }

        Ok(created)
    }
}

impl UserService {
    pub fn new(
        user_storage: Arc<GenericPostgresStorage<User>>,
        site_assignments: Arc<SiteAssignmentService>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            user_storage,
            site_assignments,
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

    /// Look up the user whose one-click unsubscribe link carries `token`.
    pub async fn get_by_unsubscribe_token(&self, token: &Uuid) -> Result<Option<User>> {
        self.user_storage
            .get_unique(StorableFilter::<User>::new_from_email_unsubscribe_token(
                token,
            ))
            .await?
            .at_most_one()
    }

    /// The token for `user`'s one-click unsubscribe links. Minted and stored
    /// the first time a user needs one, then reused, so every link the user
    /// has received keeps working.
    pub async fn ensure_unsubscribe_token(&self, user: &User) -> Result<Uuid> {
        if let Some(token) = user.base.email_unsubscribe_token {
            return Ok(token);
        }
        let token = Uuid::new_v4();
        let mut updated = user.clone();
        updated.base.email_unsubscribe_token = Some(token);
        self.update(
            &mut updated,
            AuthenticatedEntity::System,
            UpdateReason::UserUnsubscribeTokenIssued,
        )
        .await?;
        Ok(token)
    }

    /// Apply `change` to `user`'s email settings and store the result when it
    /// changed anything. Returns whether it did.
    pub async fn update_email_settings(
        &self,
        mut user: User,
        reason: UpdateReason,
        change: impl FnOnce(&mut EmailSettings) -> bool + Send,
    ) -> Result<bool> {
        if !change(&mut user.base.email_settings) {
            return Ok(false);
        }
        self.update(&mut user, AuthenticatedEntity::System, reason)
            .await?;
        Ok(true)
    }

    /// Whether this server has any user at all. A server without one is a
    /// fresh install.
    pub async fn any_exist(&self) -> Result<bool> {
        self.user_storage
            .exists(StorableFilter::<User>::new_for_server_wide_job())
            .await
    }

    /// Record that `user` has seen the release notes up to `version`.
    pub async fn mark_release_notes_seen(&self, mut user: User, version: &str) -> Result<()> {
        if user.base.release_notes_seen_version.as_deref() == Some(version) {
            return Ok(());
        }
        user.base.release_notes_seen_version = Some(version.to_string());
        self.update(
            &mut user,
            AuthenticatedEntity::System,
            UpdateReason::UserReleaseNotesSeen,
        )
        .await?;
        Ok(())
    }

    /// Record that `user` accepted the terms now.
    pub async fn accept_terms(&self, mut user: User) -> Result<User> {
        user.base.terms_accepted_at = Some(crate::server::shared::clock::now());
        let actor = AuthenticatedEntity::from(user.clone());
        self.update(&mut user, actor, UpdateReason::UserTermsAccepted)
            .await
    }

    /// Record that the user `user_id` has been emailed notice of the terms
    /// update effective on `version`. Reads the user afresh, so a long sweep
    /// never writes back a copy it fetched before the user changed it.
    pub async fn mark_terms_notified(&self, user_id: &Uuid, version: NaiveDate) -> Result<()> {
        let Some(mut user) = self.get_by_id(user_id).await? else {
            return Ok(());
        };
        if user.base.terms_notice_version >= Some(version) {
            return Ok(());
        }
        user.base.terms_notice_version = Some(version);
        self.update(
            &mut user,
            AuthenticatedEntity::System,
            UpdateReason::UserTermsNotified,
        )
        .await?;
        Ok(())
    }

    pub async fn get_organization_owners(&self, organization_id: &Uuid) -> Result<Vec<User>> {
        let filter = StorableFilter::<User>::new_from_org_id(organization_id)
            .user_permissions(&UserOrgPermissions::Owner);

        self.user_storage.get_all(filter).await
    }

    /// [`Self::get_organization_owners`] inside `tx`, for a read under the owners lock.
    pub async fn get_organization_owners_in_tx(
        &self,
        organization_id: &Uuid,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Vec<User>> {
        let filter = StorableFilter::<User>::new_from_org_id(organization_id)
            .user_permissions(&UserOrgPermissions::Owner);
        GenericPostgresStorage::<User>::get_all_in_tx(filter, tx).await
    }

    /// Read a user inside `tx`.
    pub async fn get_in_tx(
        &self,
        id: &Uuid,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Option<User>> {
        let filter = StorableFilter::<User>::new_from_entity_id(id);
        let users = GenericPostgresStorage::<User>::get_all_in_tx(filter, tx).await?;
        Ok(users.into_iter().next())
    }

    /// Hold `organization_id`'s owners lock until the guard is released. For a handler with no
    /// transaction; it reads the owners and its targets after this returns.
    pub async fn lock_organization_owners(
        &self,
        organization_id: Uuid,
    ) -> Result<SessionLockGuard> {
        Ok(self
            .user_storage
            .session_lock(
                LockKey::OrganizationOwners { organization_id },
                DEFAULT_LOCK_TIMEOUT,
            )
            .await?)
    }

    /// Hold `organization_id`'s owners lock until `tx` commits or rolls back. It contends with
    /// [`Self::lock_organization_owners`] on the same key.
    pub async fn lock_organization_owners_in_tx(
        &self,
        organization_id: Uuid,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
        xact_lock(
            tx,
            LockKey::OrganizationOwners { organization_id },
            DEFAULT_LOCK_TIMEOUT,
        )
        .await?;
        Ok(())
    }

    /// Returns all users with access to `site_id` within `organization_id`.
    ///
    /// That is every user assigned the site, directly or through All sites.
    pub async fn get_users_with_site_access(
        &self,
        site_id: &Uuid,
        organization_id: &Uuid,
    ) -> Result<Vec<User>> {
        let user_ids = self
            .site_assignments
            .user_ids_on_site(*site_id, *organization_id)
            .await?;
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        self.user_storage
            .get_all(StorableFilter::<User>::new_from_entity_ids(&user_ids))
            .await
    }

    /// The sites listed for a user. All sites lists none.
    pub async fn get_site_ids(&self, user_id: &Uuid) -> Result<Vec<Uuid>> {
        self.site_assignments
            .listed_site_ids(Principal::User(*user_id))
            .await
    }

    /// Hydrate site_ids for a single user
    pub async fn hydrate_site_ids(&self, user: &mut User) -> Result<()> {
        user.base.site_ids = self.get_site_ids(&user.id).await?;
        Ok(())
    }

    /// Hydrate site_ids for multiple users (batch operation)
    pub async fn hydrate_site_ids_batch(&self, users: &mut [User]) -> Result<()> {
        let user_ids: Vec<Uuid> = users.iter().map(|u| u.id).collect();
        let listed = self
            .site_assignments
            .listed_site_ids_by("user_id", &user_ids, |principal| match principal {
                Principal::User(id) => Some(*id),
                _ => None,
            })
            .await?;

        for user in users.iter_mut() {
            user.base.site_ids = listed.get(&user.id).cloned().unwrap_or_default();
        }

        Ok(())
    }
}
