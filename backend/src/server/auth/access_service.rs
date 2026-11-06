//! Loads a principal's access and writes it. [`AccessService::set_access`] is the only write path
//! for a principal's role and site assignments.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use sqlx::{Postgres, Transaction};
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

/// What a user with `role` holds: All sites at Admin for an Owner, otherwise `assignments`.
fn held_by_person(role: UserOrgPermissions, assignments: &[Assignment]) -> &[Assignment] {
    const EVERY_SITE: &[Assignment] = &[Assignment {
        role: SiteRole::Admin,
        scope: AssignmentScope::AllSites,
    }];
    if role == UserOrgPermissions::Owner {
        EVERY_SITE
    } else {
        assignments
    }
}

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

    /// Lock the owners of `id`'s organization in `tx` and read the user under that lock.
    async fn lock_user_in_tx(&self, id: Uuid, tx: &mut Transaction<'_, Postgres>) -> Result<User> {
        let organization_id = self
            .user_service
            .get_in_tx(&id, tx)
            .await?
            .ok_or_else(|| anyhow!("User {id} does not exist"))?
            .base
            .organization_id;
        self.user_service
            .lock_organization_owners_in_tx(organization_id, tx)
            .await?;
        let user = self
            .user_service
            .get_in_tx(&id, tx)
            .await?
            .ok_or_else(|| anyhow!("User {id} does not exist"))?;
        if user.base.organization_id != organization_id {
            return Err(ApiError::conflict("This user just changed organization.").into());
        }
        Ok(user)
    }

    /// Give `principal` this organization role and these site assignments, replacing what it held.
    /// The role and rows commit together, and the Updated event follows the commit.
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
                let mut tx = self.site_assignments.begin().await?;
                let before = self.lock_user_in_tx(id, &mut tx).await?;
                let organization_id = before.base.organization_id;
                if before.base.permissions == UserOrgPermissions::Owner
                    && role != UserOrgPermissions::Owner
                    && self
                        .user_service
                        .get_organization_owners_in_tx(&organization_id, &mut tx)
                        .await?
                        .len()
                        <= 1
                {
                    return Err(ApiError::conflict(
                        "Can't demote the only owner in an organization.",
                    )
                    .into());
                }
                let mut after = before.clone();
                after.base.permissions = role;
                let role_changed = before.base.permissions != role;
                if role_changed {
                    self.user_service.update_in_tx(&after, &mut tx).await?;
                }
                self.site_assignments
                    .replace_in_tx(
                        organization_id,
                        principal,
                        held_by_person(role, assignments),
                        &mut tx,
                    )
                    .await?;
                tx.commit().await?;
                if role_changed {
                    self.user_service
                        .publish_updated(&before, &after, actor, reason)
                        .await;
                }
            }
            Principal::ApiKey(id) => {
                let before = self
                    .user_api_key_service
                    .get_by_id(&id)
                    .await?
                    .ok_or_else(|| anyhow!("API key {id} does not exist"))?;
                let mut after = before.clone();
                after.base.permissions = role;
                let role_changed = before.base.permissions != role;
                let mut tx = self.site_assignments.begin().await?;
                if role_changed {
                    self.user_api_key_service
                        .update_in_tx(&after, &mut tx)
                        .await?;
                }
                self.site_assignments
                    .replace_in_tx(before.base.organization_id, principal, assignments, &mut tx)
                    .await?;
                tx.commit().await?;
                if role_changed {
                    self.user_api_key_service
                        .publish_updated(&before, &after, actor, reason)
                        .await;
                }
            }
            Principal::Invite(id) => {
                let before = self
                    .invite_service
                    .get_by_id(&id)
                    .await?
                    .ok_or_else(|| anyhow!("Invite {id} does not exist"))?;
                let mut after = before.clone();
                after.base.permissions = role;
                let role_changed = before.base.permissions != role;
                let mut tx = self.site_assignments.begin().await?;
                if role_changed {
                    self.invite_service.update_in_tx(&after, &mut tx).await?;
                }
                self.site_assignments
                    .replace_in_tx(before.base.organization_id, principal, assignments, &mut tx)
                    .await?;
                tx.commit().await?;
                if role_changed {
                    self.invite_service
                        .publish_updated(&before, &after, actor, reason)
                        .await;
                }
            }
        }
        Ok(())
    }

    /// Move a user into `organization_id` with this role and these site assignments. The
    /// organization, role and rows commit together, and the Updated event follows the commit.
    ///
    /// Takes the owners lock of the organization the user leaves. Nothing here refuses the last
    /// Owner leaving.
    pub async fn join_organization(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        role: UserOrgPermissions,
        assignments: &[Assignment],
        actor: AuthenticatedEntity,
    ) -> Result<()> {
        let mut tx = self.site_assignments.begin().await?;
        let before = self.lock_user_in_tx(user_id, &mut tx).await?;
        let mut after = before.clone();
        after.base.organization_id = organization_id;
        after.base.permissions = role;
        self.user_service.update_in_tx(&after, &mut tx).await?;
        self.site_assignments
            .replace_in_tx(
                organization_id,
                Principal::User(user_id),
                held_by_person(role, assignments),
                &mut tx,
            )
            .await?;
        tx.commit().await?;
        self.user_service
            .publish_updated(&before, &after, actor, UpdateReason::UserJoinedOrganization)
            .await;
        Ok(())
    }

    /// Store a key and give it its role on `site_ids`, or on All sites when `None`. Announced once
    /// its access is saved, so a failed save leaves no "created" event behind.
    pub async fn create_api_key(
        &self,
        api_key: UserApiKey,
        site_ids: Option<Vec<Uuid>>,
        actor: AuthenticatedEntity,
    ) -> Result<UserApiKey> {
        let role = api_key.base.permissions;
        let service = &self.user_api_key_service;
        let mut created = service.store_created(api_key, &actor).await?;
        self.set_access(
            Principal::ApiKey(created.id),
            role,
            &Assignment::for_api_key(role, site_ids.as_deref()),
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
        // An All sites assignment at Admin already covers the new site, so it adds no row.
        let admin = SiteRole::Admin.grants();
        let covered = self
            .site_assignments
            .get_for(principal)
            .await?
            .iter()
            .any(|held| {
                held.scope == AssignmentScope::AllSites && admin.is_subset_of(&held.role.grants())
            });
        if covered {
            return Ok(());
        }
        self.site_assignments
            .add(organization_id, principal, assignment)
            .await
    }
}
