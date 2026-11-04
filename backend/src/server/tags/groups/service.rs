use crate::server::shared::events::types::{DeletionReason, EntityOperation, UpdateReason};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        entities::EntityDiscriminants,
        events::{bus::EventBus, traits::Event},
        services::traits::{CrudService, EventBusService},
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
            traits::{Storable, Storage},
        },
        types::api::ValidationError,
    },
    tags::{
        entity_tags::EntityTag,
        groups::base::{TagGroup, TagGroupBase, TagGroupType},
        r#impl::base::Tag,
        service::TagService,
    },
};

/// Tag groups: the system groups every organization has, the groups it names, each group's tag
/// order, and the entity types a group's tags may be applied to.
///
/// A group's rules are rules about its tags and their assignments, which it reads and writes
/// through `TagService`.
pub struct TagGroupService {
    storage: Arc<GenericPostgresStorage<TagGroup>>,
    event_bus: Arc<EventBus>,
    /// Set after construction: `TagService` is built from this service.
    tag_service: OnceLock<Arc<TagService>>,
}

impl EventBusService<TagGroup> for TagGroupService {
    fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    fn get_site_id(&self, _entity: &TagGroup) -> Option<Uuid> {
        None
    }
    fn get_organization_id(&self, entity: &TagGroup) -> Option<Uuid> {
        Some(entity.base.organization_id)
    }
}

#[async_trait]
impl CrudService<TagGroup> for TagGroupService {
    fn storage(&self) -> &Arc<GenericPostgresStorage<TagGroup>> {
        &self.storage
    }

    fn entity_tag_service(
        &self,
    ) -> Option<&Arc<crate::server::tags::entity_tags::EntityTagService>> {
        None
    }

    /// Groups created through the API are always named by the organization.
    async fn create(
        &self,
        mut entity: TagGroup,
        authentication: AuthenticatedEntity,
    ) -> Result<TagGroup> {
        entity.base.group_type = TagGroupType::Custom;
        entity.base.name = entity.base.name.trim().to_string();
        self.refuse_taken_name(&entity).await?;
        self.create_base(entity, authentication).await
    }

    /// A system group keeps its name and type. Narrowing the entity types is refused while the
    /// group's tags are applied to a type it would no longer allow.
    async fn update(
        &self,
        entity: &mut TagGroup,
        authentication: AuthenticatedEntity,
        reason: UpdateReason,
    ) -> Result<TagGroup> {
        let current = self
            .get_by_id(&entity.id)
            .await?
            .ok_or_else(|| anyhow!("Tag group {} not found", entity.id))?;

        entity.base.group_type = current.base.group_type;
        entity.base.organization_id = current.base.organization_id;
        if current.is_system() {
            entity.base.ordered = current.base.ordered;
        }
        if let Some(types) = current.base.group_type.computed_entity_types() {
            entity.base.entity_types = Some(types);
        }
        entity.base.name = entity.base.name.trim().to_string();
        if current.is_system() && entity.base.name != current.base.name {
            return Err(ValidationError::new(format!(
                "The \"{}\" group is a system group and can't be renamed",
                current.base.name
            ))
            .into());
        }
        if !entity.base.name.eq_ignore_ascii_case(&current.base.name) {
            self.refuse_taken_name(entity).await?;
        }
        self.refuse_narrowing_in_use(&current, &entity.base).await?;

        let mut updated = entity.clone();
        updated.created_at = current.created_at;
        updated.updated_at = crate::server::shared::clock::now();
        self.storage.update(&mut updated).await?;
        if updated.base.ordered != current.base.ordered {
            self.renumber_tags(&updated).await?;
        }
        self.publish_updated(&current, &updated, authentication, reason)
            .await;
        Ok(updated)
    }

    /// Deleting a group leaves its tags in no group. System groups can't be deleted.
    async fn delete(
        &self,
        id: &Uuid,
        authentication: AuthenticatedEntity,
        reason: DeletionReason,
    ) -> Result<()> {
        let group = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow!("Tag group {id} not found"))?;
        self.refuse_system_delete(&group)?;
        self.ungroup_tags(&group).await?;
        self.storage.delete(id).await?;
        self.publish_change(&group, authentication, EntityOperation::Deleted { reason })
            .await?;
        Ok(())
    }

    async fn delete_many(
        &self,
        ids: &[Uuid],
        authentication: AuthenticatedEntity,
        reason: DeletionReason,
    ) -> Result<usize> {
        let groups = self
            .get_all(StorableFilter::<TagGroup>::new_from_entity_ids(ids))
            .await?;
        for group in &groups {
            self.refuse_system_delete(group)?;
        }
        for group in &groups {
            self.delete(&group.id, authentication.clone(), reason.clone())
                .await?;
        }
        Ok(groups.len())
    }
}

impl TagGroupService {
    pub fn new(storage: Arc<GenericPostgresStorage<TagGroup>>, event_bus: Arc<EventBus>) -> Self {
        Self {
            storage,
            event_bus,
            tag_service: OnceLock::new(),
        }
    }

    /// Set the tag service dependency after construction (breaks circular dep).
    pub fn set_tag_service(&self, service: Arc<TagService>) -> Result<(), Arc<TagService>> {
        self.tag_service.set(service)
    }

    fn tag_service(&self) -> Result<&Arc<TagService>> {
        self.tag_service
            .get()
            .ok_or_else(|| anyhow!("TagService not initialized"))
    }

    async fn publish_change(
        &self,
        group: &TagGroup,
        authentication: AuthenticatedEntity,
        operation: EntityOperation,
    ) -> Result<()> {
        use crate::server::shared::events::traits::EntityScope;
        if let Some(scope) = EntityScope::from_ids(
            group.id,
            group.clone().into(),
            None,
            Some(group.base.organization_id),
        ) {
            self.event_bus
                .publish(Event::new(scope, operation, authentication))
                .await;
        }
        Ok(())
    }

    /// Every group of the organization: system groups first, then by name.
    pub async fn get_for_org(&self, organization_id: &Uuid) -> Result<Vec<TagGroup>> {
        let mut groups = self
            .get_all(StorableFilter::<TagGroup>::new_from_org_id(organization_id))
            .await?;
        groups.sort_by(|a, b| {
            (
                !a.is_system(),
                a.base.group_type.to_string(),
                a.base.name.to_lowercase(),
            )
                .cmp(&(
                    !b.is_system(),
                    b.base.group_type.to_string(),
                    b.base.name.to_lowercase(),
                ))
        });
        Ok(groups)
    }

    /// The organization's system group of `group_type`, if it has one.
    pub async fn system_group(
        &self,
        organization_id: &Uuid,
        group_type: TagGroupType,
    ) -> Result<Option<TagGroup>> {
        Ok(self
            .get_for_org(organization_id)
            .await?
            .into_iter()
            .find(|g| g.base.group_type == group_type && g.is_system()))
    }

    /// Create whichever of the system groups and the `custom` named groups the organization
    /// lacks, and return every group it then has. Serialized per organization, so two callers
    /// can't both create the same group.
    pub async fn seed_groups(
        &self,
        organization_id: Uuid,
        custom: &[&str],
        authentication: AuthenticatedEntity,
    ) -> Result<Vec<TagGroup>> {
        let lock = self
            .storage
            .session_lock(LockKey::TagGroups { organization_id }, DEFAULT_LOCK_TIMEOUT)
            .await?;

        let existing = self.get_for_org(&organization_id).await?;
        let mut missing: Vec<TagGroupBase> = TagGroupType::system()
            .filter(|t| !existing.iter().any(|g| g.base.group_type == *t))
            .map(|t| TagGroupBase::new(organization_id, t.to_string(), t))
            .collect();
        missing.extend(
            custom
                .iter()
                .filter(|name| {
                    !existing
                        .iter()
                        .any(|g| g.base.name.eq_ignore_ascii_case(name))
                })
                .map(|name| TagGroupBase {
                    description:
                        crate::server::shared::storage::seed_data::starter_group_description(name)
                            .map(str::to_string),
                    ..TagGroupBase::new(organization_id, name.to_string(), TagGroupType::Custom)
                }),
        );
        for base in missing {
            let group = self
                .create_base(TagGroup::new(base), authentication.clone())
                .await?;
            // A new Visibility group starts with the tag that hides a host.
            if group.base.group_type == TagGroupType::Visibility {
                self.tag_service()?
                    .store_seed_tag(&crate::server::shared::storage::seed_data::hidden_tag(
                        organization_id,
                        group.id,
                    ))
                    .await?;
            }
        }

        lock.release().await?;
        self.get_for_org(&organization_id).await
    }

    /// The position a tag joining `group` takes: after every live tag already in it, or none when
    /// the group is unordered.
    pub async fn next_position(&self, group: &TagGroup) -> Result<Option<i32>> {
        if !group.base.ordered {
            return Ok(None);
        }
        let tags = self.live_tags_in(group.id).await?;
        let last = tags
            .iter()
            .filter_map(|t| t.base.position)
            .max()
            .unwrap_or(0);
        Ok(Some(last + 1))
    }

    /// After `group` turns ordered or unordered: an ordered group numbers its tags by name, an
    /// unordered one clears their positions.
    async fn renumber_tags(&self, group: &TagGroup) -> Result<()> {
        let mut tags = self.live_tags_in(group.id).await?;
        tags.sort_by_key(|t| t.base.name.to_lowercase());
        for (index, tag) in tags.iter_mut().enumerate() {
            tag.base.position = group.base.ordered.then_some(index as i32 + 1);
        }
        self.tag_service()?.store_group_changes(&tags).await
    }

    async fn live_tags_in(&self, group_id: Uuid) -> Result<Vec<Tag>> {
        self.tag_service()?.live_tags_in_group(group_id).await
    }

    /// Set the order of a group's tags. `tag_ids` must name every live tag of the group, once.
    pub async fn reorder_tags(&self, group: &TagGroup, tag_ids: &[Uuid]) -> Result<Vec<Tag>> {
        if !group.base.ordered {
            return Err(ValidationError::new(format!(
                "The \"{}\" group lists its tags by name; it has no order to set",
                group.base.name
            ))
            .into());
        }
        self.tag_service()?.reorder_in_group(group, tag_ids).await
    }

    /// Refuse to apply `tags` to `entity_type` when one of their groups doesn't allow it.
    pub async fn refuse_out_of_scope(
        &self,
        tags: &[Tag],
        entity_type: EntityDiscriminants,
    ) -> Result<()> {
        let group_ids: Vec<Uuid> = tags.iter().filter_map(|t| t.base.tag_group_id).collect();
        if group_ids.is_empty() {
            return Ok(());
        }
        let groups = self
            .get_all(StorableFilter::<TagGroup>::new_from_entity_ids(&group_ids))
            .await?;
        for tag in tags {
            let Some(group) = groups.iter().find(|g| Some(g.id) == tag.base.tag_group_id) else {
                continue;
            };
            if !group.base.allows(entity_type) {
                return Err(ValidationError::new(format!(
                    "Tags in the \"{}\" group can't be applied to a {}",
                    group.base.name,
                    entity_type.entity_name_singular()
                ))
                .into());
            }
        }
        Ok(())
    }

    /// Refuse to move `tag` into `group` while it is applied to an entity type the group doesn't
    /// allow.
    pub async fn refuse_tag_out_of_scope(&self, tag: &Tag, group: &TagGroup) -> Result<()> {
        let rows = self.tag_service()?.live_assignments(&[tag.id]).await?;
        let removed: Vec<EntityDiscriminants> = rows
            .iter()
            .map(|r| r.base.entity_type)
            .filter(|t| !group.base.allows(*t))
            .collect();
        match counts_by_type(&rows, &removed) {
            None => Ok(()),
            Some(counts) => Err(ValidationError::new(format!(
                "The \"{}\" group doesn't allow every entity this tag is on: {counts}. Remove the tag from them first.",
                group.base.name
            ))
            .into()),
        }
    }

    async fn refuse_narrowing_in_use(&self, current: &TagGroup, next: &TagGroupBase) -> Result<()> {
        let removed = current.base.types_removed_by(next);
        if removed.is_empty() {
            return Ok(());
        }
        let tag_ids: Vec<Uuid> = self
            .live_tags_in(current.id)
            .await?
            .iter()
            .map(|t| t.id)
            .collect();
        let rows = self.tag_service()?.live_assignments(&tag_ids).await?;
        match counts_by_type(&rows, &removed) {
            None => Ok(()),
            Some(counts) => Err(ValidationError::new(format!(
                "Tags in the \"{}\" group are still applied to {counts}. Remove them first.",
                current.base.name
            ))
            .into()),
        }
    }

    async fn refuse_taken_name(&self, group: &TagGroup) -> Result<()> {
        let taken = self
            .get_for_org(&group.base.organization_id)
            .await?
            .iter()
            .any(|g| g.id != group.id && g.base.name.eq_ignore_ascii_case(&group.base.name));
        if taken {
            return Err(ValidationError::new(format!(
                "A tag group named \"{}\" already exists",
                group.base.name
            ))
            .into());
        }
        Ok(())
    }

    fn refuse_system_delete(&self, group: &TagGroup) -> Result<()> {
        if group.is_system() {
            return Err(ValidationError::new(format!(
                "The \"{}\" group is a system group and can't be deleted",
                group.base.name
            ))
            .into());
        }
        Ok(())
    }

    /// Take every live tag out of `group`. Closed and snapshot rows keep their reference until
    /// the row is deleted, when the foreign key clears it.
    async fn ungroup_tags(&self, group: &TagGroup) -> Result<()> {
        let mut tags = self.live_tags_in(group.id).await?;
        for tag in tags.iter_mut() {
            tag.base.tag_group_id = None;
            tag.base.position = None;
        }
        self.tag_service()?.store_group_changes(&tags).await
    }
}

/// Whether `ids` holds exactly the members of `expected`, each once.
pub(crate) fn is_permutation(ids: &[Uuid], expected: &[Uuid]) -> bool {
    let mut a = ids.to_vec();
    let mut b = expected.to_vec();
    a.sort();
    b.sort();
    let unique = {
        let mut d = a.clone();
        d.dedup();
        d.len() == a.len()
    };
    unique && a == b
}

/// How many live assignments in `rows` fall on each of `types`.
fn assignments_by_type(
    rows: &[EntityTag],
    types: &[EntityDiscriminants],
) -> BTreeMap<EntityDiscriminants, usize> {
    let mut counts: BTreeMap<EntityDiscriminants, usize> = BTreeMap::new();
    for row in rows.iter().filter(|r| types.contains(&r.base.entity_type)) {
        *counts.entry(row.base.entity_type).or_default() += 1;
    }
    counts
}

/// "3 hosts, 1 service": the assignments in `rows` on each of `types`. `None` when none are.
fn counts_by_type(rows: &[EntityTag], types: &[EntityDiscriminants]) -> Option<String> {
    let counts = assignments_by_type(rows, types);
    if counts.is_empty() {
        return None;
    }
    Some(
        counts
            .iter()
            .map(|(entity_type, count)| {
                let name = if *count == 1 {
                    entity_type.entity_name_singular()
                } else {
                    entity_type.entity_name_plural()
                };
                format!("{count} {}", name.to_lowercase())
            })
            .collect::<Vec<_>>()
            .join(", "),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::tags::entity_tags::EntityTagBase;

    #[test]
    fn a_reorder_must_name_each_tag_once() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        assert!(is_permutation(&[b, a], &[a, b]));
        assert!(!is_permutation(&[a], &[a, b]));
        assert!(!is_permutation(&[a, a], &[a, b]));
        assert!(!is_permutation(&[a, b, c], &[a, b]));
    }

    #[test]
    fn counts_name_only_the_types_asked_about() {
        let tag = Uuid::new_v4();
        let row =
            |entity_type| EntityTag::new(EntityTagBase::new(Uuid::new_v4(), entity_type, tag));
        let rows = [
            row(EntityDiscriminants::Host),
            row(EntityDiscriminants::Host),
            row(EntityDiscriminants::Service),
            row(EntityDiscriminants::Vlan),
        ];
        let counts = assignments_by_type(
            &rows,
            &[EntityDiscriminants::Host, EntityDiscriminants::Service],
        );
        assert_eq!(
            counts.into_iter().collect::<Vec<_>>(),
            vec![
                (EntityDiscriminants::Host, 2),
                (EntityDiscriminants::Service, 1)
            ]
        );
        assert_eq!(counts_by_type(&rows, &[EntityDiscriminants::Subnet]), None);
    }
}
