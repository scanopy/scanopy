//! Loads a principal's access and writes it. [`AccessService::set_access`] is the only write path
//! for a principal's role and site assignments.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use uuid::Uuid;

use crate::server::{
    auth::{
        access::{AccessSet, Assignment, AssignmentScope, SiteRole},
        middleware::auth::AuthenticatedEntity,
    },
    invites::{r#impl::base::Invite, service::InviteService},
    shared::{
        events::types::UpdateReason, services::traits::CrudService,
        storage::filter::StorableFilter, types::api::ApiError,
    },
    site_assignments::{r#impl::base::Principal, service::SiteAssignmentService},
    sites::{r#impl::Site, service::SiteService},
    user_api_keys::{r#impl::base::UserApiKey, service::UserApiKeyService},
    users::{
        r#impl::{base::User, permissions::UserOrgPermissions},
        service::UserService,
    },
};

pub struct AccessService {
    site_assignments: Arc<SiteAssignmentService>,
    user_service: Arc<UserService>,
    user_api_key_service: Arc<UserApiKeyService>,
    invite_service: Arc<InviteService>,
    site_service: Arc<SiteService>,
}

impl AccessService {
    pub fn new(
        site_assignments: Arc<SiteAssignmentService>,
        user_service: Arc<UserService>,
        user_api_key_service: Arc<UserApiKeyService>,
        invite_service: Arc<InviteService>,
        site_service: Arc<SiteService>,
    ) -> Self {
        Self {
            site_assignments,
            user_service,
            user_api_key_service,
            invite_service,
            site_service,
        }
    }

    /// What `role` and `assignments` come to in `organization_id` today.
    pub async fn resolve(
        &self,
        organization_id: Uuid,
        role: UserOrgPermissions,
        assignments: &[Assignment],
    ) -> Result<AccessSet> {
        let covers_all_sites = assignments
            .iter()
            .any(|assignment| assignment.scope == AssignmentScope::AllSites);
        let organization_site_ids: Vec<Uuid> = if covers_all_sites {
            self.site_service
                .get_all(StorableFilter::<Site>::new_from_org_id(&organization_id))
                .await?
                .iter()
                .map(|site| site.id)
                .collect()
        } else {
            Vec::new()
        };
        Ok(AccessSet::new(role, assignments, &organization_site_ids))
    }

    /// What `user` can do.
    pub async fn user_access(&self, user: &User) -> Result<AccessSet> {
        let assignments = self
            .site_assignments
            .get_for(Principal::User(user.id))
            .await?;
        self.resolve(
            user.base.organization_id,
            user.base.permissions,
            &assignments,
        )
        .await
    }

    /// What `key` can do: its own access, capped by what its owner can do now, so a demoted owner's
    /// keys drop with them. `None` when the owner is gone.
    pub async fn api_key_access(&self, key: &UserApiKey) -> Result<Option<AccessSet>> {
        let Some(owner) = self.user_service.get_by_id(&key.base.user_id).await? else {
            return Ok(None);
        };
        let owner_access = self.user_access(&owner).await?;
        let assignments = self
            .site_assignments
            .get_for(Principal::ApiKey(key.id))
            .await?;
        let access = self
            .resolve(key.base.organization_id, key.base.permissions, &assignments)
            .await?;
        Ok(Some(access.capped_by(&owner_access)))
    }

    /// What accepting `invite` would give.
    pub async fn invite_access(&self, invite: &Invite) -> Result<AccessSet> {
        let assignments = self
            .site_assignments
            .get_for(Principal::Invite(invite.id))
            .await?;
        self.resolve(
            invite.base.organization_id,
            invite.base.permissions,
            &assignments,
        )
        .await
    }

    /// Give `principal` this organization role and these site assignments, replacing what it held.
    ///
    /// The last Owner can't be demoted, and a user with org Owner always holds All sites at Admin,
    /// whatever `assignments` says. Keys have neither rule, so a limited Owner key stays limited.
    pub async fn set_access(
        &self,
        principal: Principal,
        role: UserOrgPermissions,
        assignments: &[Assignment],
        actor: AuthenticatedEntity,
    ) -> Result<()> {
        let reason = UpdateReason::EntityRequested;
        match principal {
            Principal::User(id) => {
                let mut user = self
                    .user_service
                    .get_by_id(&id)
                    .await?
                    .ok_or_else(|| anyhow!("User {id} does not exist"))?;
                let organization_id = user.base.organization_id;
                if user.base.permissions == UserOrgPermissions::Owner
                    && role != UserOrgPermissions::Owner
                    && self
                        .user_service
                        .get_organization_owners(&organization_id)
                        .await?
                        .len()
                        <= 1
                {
                    return Err(ApiError::conflict(
                        "Can't demote the only owner in an organization.",
                    )
                    .into());
                }
                let every_site = [Assignment {
                    role: SiteRole::Admin,
                    scope: AssignmentScope::AllSites,
                }];
                let assignments = if role == UserOrgPermissions::Owner {
                    &every_site[..]
                } else {
                    assignments
                };
                self.site_assignments
                    .replace(organization_id, principal, assignments)
                    .await?;
                if user.base.permissions != role {
                    user.base.permissions = role;
                    self.user_service.update(&mut user, actor, reason).await?;
                }
            }
            Principal::ApiKey(id) => {
                let mut key = self
                    .user_api_key_service
                    .get_by_id(&id)
                    .await?
                    .ok_or_else(|| anyhow!("API key {id} does not exist"))?;
                self.site_assignments
                    .replace(key.base.organization_id, principal, assignments)
                    .await?;
                if key.base.permissions != role {
                    key.base.permissions = role;
                    self.user_api_key_service
                        .update(&mut key, actor, reason)
                        .await?;
                }
            }
            Principal::Invite(id) => {
                let mut invite = self
                    .invite_service
                    .get_by_id(&id)
                    .await?
                    .ok_or_else(|| anyhow!("Invite {id} does not exist"))?;
                self.site_assignments
                    .replace(invite.base.organization_id, principal, assignments)
                    .await?;
                if invite.base.permissions != role {
                    invite.base.permissions = role;
                    self.invite_service
                        .update(&mut invite, actor, reason)
                        .await?;
                }
            }
        }
        Ok(())
    }

    /// Store a key and give it its role on `site_ids`. Announced once its access is saved, so a
    /// failed save leaves no "created" event behind.
    pub async fn create_api_key(
        &self,
        api_key: UserApiKey,
        site_ids: Vec<Uuid>,
        actor: AuthenticatedEntity,
    ) -> Result<UserApiKey> {
        let role = api_key.base.permissions;
        let service = &self.user_api_key_service;
        let mut created = service.store_created(api_key, &actor).await?;
        self.set_access(
            Principal::ApiKey(created.id),
            role,
            &Assignment::for_api_key(role, &site_ids),
            actor.clone(),
        )
        .await?;
        created.base.site_ids = site_ids;
        service.publish_created(&created, actor).await;
        Ok(created)
    }

    /// Store an invite and give it what `permissions` plus `site_ids` mean on a person.
    pub async fn create_invite(
        &self,
        invite: Invite,
        actor: AuthenticatedEntity,
    ) -> Result<Invite> {
        let role = invite.base.permissions;
        let assignments = Assignment::for_person(role, &invite.base.site_ids);
        let service = &self.invite_service;
        let mut created = service.store_created(invite, &actor).await?;
        self.set_access(
            Principal::Invite(created.id),
            role,
            &assignments,
            actor.clone(),
        )
        .await?;
        service.hydrate_site_ids(&mut created).await?;
        service.publish_created(&created, actor).await;
        Ok(created)
    }

    /// Give whoever created `site_id` an Admin assignment on it. Everyone else holds no assignment
    /// to keep: a daemon or the system creates sites on someone else's behalf.
    pub async fn keep_creator_access(
        &self,
        creator: &AuthenticatedEntity,
        organization_id: Uuid,
        site_id: Uuid,
    ) -> Result<()> {
        let principal = match creator {
            AuthenticatedEntity::User { user_id, .. } => Principal::User(*user_id),
            AuthenticatedEntity::ApiKey { api_key_id, .. } => Principal::ApiKey(*api_key_id),
            _ => return Ok(()),
        };
        let assignment = Assignment {
            role: SiteRole::Admin,
            scope: AssignmentScope::Site(site_id),
        };
        self.site_assignments
            .add(organization_id, principal, assignment)
            .await
    }
}
