use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        entities::ChangeTriggersTopologyStaleness,
        events::{
            bus::EventBus,
            traits::{EntityEventFlags, EntityScope, Event},
            types::EntityOperation,
        },
        services::traits::{CrudService, EventBusService, SnapshotMutator},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            traits::{Entity, Storage},
        },
        types::api::ValidationError,
    },
    tags::{
        entity_tags::{EntityTagStorage, entities_holding_several},
        r#impl::base::{ExclusiveSet, Tag},
    },
};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

pub struct TagService {
    storage: Arc<GenericPostgresStorage<Tag>>,
    /// The tags module's own junction storage, read to check a set change against existing
    /// assignments.
    entity_tags: Arc<EntityTagStorage>,
    event_bus: Arc<EventBus>,
}

impl EventBusService<Tag> for TagService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, _entity: &Tag) -> Option<Uuid> {
        None
    }
    fn get_organization_id(&self, entity: &Tag) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }
}

#[async_trait]
impl CrudService<Tag> for TagService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<Tag>> {
        &self.storage
    }

    fn entity_tag_service(
        &self,
    ) -> Option<&Arc<crate::server::tags::entity_tags::EntityTagService>> {
        None
    }

    /// Tag rename uses per-action close-and-clone (`SnapshotMutator`) instead
    /// of in-place UPDATE: the prior field values are preserved on a closed
    /// historical row with `lineage_id` lineage; the live row keeps its id
    /// and gets the new field values plus advanced `valid_from`. Stable ids
    /// mean `entity_tags.tag_id` references survive renames without cascade
    /// UPDATE, and as-of joins through the OR-pattern resolve the right name
    /// at any point in time.
    async fn update(
        &self,
        entity: &mut Tag,
        authentication: AuthenticatedEntity,
    ) -> Result<Tag, anyhow::Error> {
        let current = self
            .get_by_id(&entity.id)
            .await?
            .ok_or_else(|| anyhow!("Could not find Tag {}", entity.id))?;

        if entity.base.exclusive_set != current.base.exclusive_set
            && let Some(set) = &entity.base.exclusive_set
        {
            self.refuse_set_already_doubled(entity, set).await?;
        }

        let updated = SnapshotMutator::close_and_clone(self, entity.clone()).await?;

        let trigger_stale = updated.triggers_staleness(Some(current));

        if let Some(scope) = EntityScope::from_ids(
            updated.id(),
            updated.clone().into(),
            self.get_site_id(&updated),
            self.get_organization_id(&updated),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Updated, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        Ok(updated)
    }

    /// Tag delete is a soft-close, not a hard delete: setting `valid_to =
    /// NOW()` on the live row preserves FK integrity for any historical
    /// `entity_tags` row that references this tag. Hard-delete would break
    /// snapshot reads of associations that existed before the delete.
    async fn delete(
        &self,
        id: &Uuid,
        authentication: AuthenticatedEntity,
    ) -> Result<(), anyhow::Error> {
        let mut entity = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow!("Tag {} not found", id))?;

        if entity.valid_to.is_some() {
            // Already soft-closed; nothing to do.
            return Ok(());
        }

        entity.valid_to = Some(chrono::Utc::now());
        let _ = self.storage().update(&mut entity).await?;

        let trigger_stale = entity.triggers_staleness(None);
        let entity_for_event = entity.clone();

        if let Some(scope) = EntityScope::from_ids(
            entity_for_event.id(),
            entity_for_event.into(),
            self.get_site_id(&entity),
            self.get_organization_id(&entity),
        ) {
            self.event_bus()
                .publish(
                    Event::new(scope, EntityOperation::Deleted, authentication).with_flags(
                        EntityEventFlags {
                            trigger_stale,
                            ..Default::default()
                        },
                    ),
                )
                .await?;
        }

        Ok(())
    }
}

impl TagService {
    pub fn new(
        storage: Arc<GenericPostgresStorage<Tag>>,
        entity_tags: Arc<EntityTagStorage>,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            storage,
            entity_tags,
            event_bus,
        }
    }

    /// Refuse to move `tag` into `set` while some entity holds both it and another tag of `set`.
    ///
    /// Moving it would leave that entity breaking the rule the set exists to keep, and which of
    /// its tags should go is a person's call. So the change is refused with a count, and nothing
    /// is removed on the person's behalf.
    async fn refuse_set_already_doubled(&self, tag: &Tag, set: &ExclusiveSet) -> Result<()> {
        let siblings = self
            .get_all(StorableFilter::<Tag>::new_from_org_id(&tag.base.organization_id).live())
            .await?;
        let mut set_tag_ids: Vec<Uuid> = siblings
            .iter()
            .filter(|t| t.id != tag.id && t.base.exclusive_set.as_ref() == Some(set))
            .map(|t| t.id)
            .collect();
        if set_tag_ids.is_empty() {
            return Ok(());
        }
        set_tag_ids.push(tag.id);

        let rows = self.entity_tags.get_live_for_tags(&set_tag_ids).await?;
        match entities_holding_several(&rows, &set_tag_ids) {
            0 => Ok(()),
            count => Err(ValidationError::new(format!(
                "{count} {} hold more than one tag in the \"{set}\" set. Remove the extra tags first.",
                if count == 1 { "entity" } else { "entities" }
            ))
            .into()),
        }
    }
}
