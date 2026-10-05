use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::server::shared::events::traits::{EntityEventFlags, EntityScope, Event};
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    invites::r#impl::base::Invite,
    shared::{
        entities::ChangeTriggersTopologyStaleness,
        events::{bus::EventBus, types::EntityOperation},
        services::traits::{CrudService, EventBusService},
        storage::{filter::StorableFilter, generic::GenericPostgresStorage, traits::Storage},
    },
    tags::entity_tags::EntityTagService,
};

pub struct InviteService {
    storage: Arc<GenericPostgresStorage<Invite>>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<Invite> for InviteService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, _entity: &Invite) -> Option<Uuid> {
        None
    }

    fn get_organization_id(&self, entity: &Invite) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }
}

impl CrudService<Invite> for InviteService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Invite>> {
        &self.storage
    }

    fn entity_tag_service(&self) -> Option<&Arc<EntityTagService>> {
        None
    }
}

impl InviteService {
    pub fn new(storage: Arc<GenericPostgresStorage<Invite>>, event_bus: Arc<EventBus>) -> Self {
        Self { storage, event_bus }
    }

    /// Get an invite by ID, validating it hasn't expired
    pub async fn get_valid_invite(&self, id: Uuid) -> Result<Invite, anyhow::Error> {
        let invite = self
            .storage
            .get_by_id(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Invalid or expired invite link"))?;

        if !invite.is_valid() {
            return Err(anyhow::anyhow!(
                "Invite link has expired or reached maximum uses"
            ));
        }

        Ok(invite)
    }

    /// Use (consume) an invite - validates and deletes it
    pub async fn use_invite(&self, id: Uuid) -> Result<Invite, anyhow::Error> {
        let invite = self
            .storage
            .get_by_id(&id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Invalid or expired invite link"))?;

        if !invite.is_valid() {
            return Err(anyhow::anyhow!("Invite link has expired"));
        }

        self.storage.delete(&id).await?;

        let trigger_stale = invite.triggers_staleness(None);

        if let Some(scope) = EntityScope::from_ids(
            invite.id,
            invite.clone().into(),
            None,
            Some(invite.base.organization_id),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Deleted, AuthenticatedEntity::System)
                        .with_flags(EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        }),
                )
                .await?;
        }

        Ok(invite)
    }

    /// Clean up expired invites from the database
    pub async fn cleanup_expired(&self) {
        let now = Utc::now();
        let filter = StorableFilter::<Invite>::new_with_expiry_before(now);

        match self.storage.get_all(filter).await {
            Ok(expired_invites) => {
                if expired_invites.is_empty() {
                    return;
                }

                let ids: Vec<Uuid> = expired_invites.iter().map(|i| i.id).collect();
                match self.storage.delete_many(&ids).await {
                    Ok(count) => {
                        tracing::debug!("Cleaned up {} expired invites.", count);
                    }
                    Err(e) => {
                        tracing::error!("Failed to delete expired invites: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to query expired invites: {}", e);
            }
        }
    }

    /// Revoke all invites for an organization
    pub async fn revoke_org_invites(&self, organization_id: &Uuid) -> Result<(), anyhow::Error> {
        let filter = StorableFilter::<Invite>::new_from_org_id(organization_id);
        let org_invites = self.storage.get_all(filter).await?;

        if !org_invites.is_empty() {
            let ids: Vec<Uuid> = org_invites.iter().map(|i| i.id).collect();
            self.storage.delete_many(&ids).await?;
        }

        Ok(())
    }

    /// Get all invites for an organization (including expired)
    pub async fn get_org_invites(
        &self,
        organization_id: &Uuid,
    ) -> Result<Vec<Invite>, anyhow::Error> {
        let filter = StorableFilter::<Invite>::new_from_org_id(organization_id);
        self.storage.get_all(filter).await
    }

    /// List all active (non-expired) invites for an organization
    pub async fn list_active_invites(&self, organization_id: &Uuid) -> Vec<Invite> {
        let filter = StorableFilter::<Invite>::new_from_org_id(organization_id);

        match self.storage.get_all(filter).await {
            Ok(invites) => invites
                .into_iter()
                .filter(|invite| invite.is_valid())
                .collect(),
            Err(e) => {
                tracing::error!("Failed to list invites: {}", e);
                Vec::new()
            }
        }
    }
}
