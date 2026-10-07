//! What the global search asks each entity service: the caller it searches for, and the text and
//! tags it searches with. Each service answers through [`CrudService::search`], whose default
//! covers every entity listed under the generic site/organization rule.
//!
//! [`CrudService::search`]: super::traits::CrudService::search

use uuid::Uuid;

use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::storage::{filter::StorableFilter, traits::Entity},
    users::r#impl::permissions::UserOrgPermissions,
};

/// Whose search this is: the rows a list endpoint would show this caller.
#[derive(Debug, Clone)]
pub struct SearchScope {
    pub site_ids: Vec<Uuid>,
    pub organization_id: Uuid,
    pub permissions: UserOrgPermissions,
    /// The signed-in user, for entities only their owner lists (user API keys). `None` for an API
    /// key, which those lists refuse.
    pub session_user_id: Option<Uuid>,
    /// The user the caller acts as, signed in or through an API key.
    pub user_id: Uuid,
}

impl SearchScope {
    /// The scope of a user or API key. `None` for a caller with no organization (a daemon).
    pub fn for_entity(entity: &AuthenticatedEntity) -> Option<Self> {
        Some(Self {
            site_ids: entity.site_ids(),
            organization_id: entity.organization_id()?,
            permissions: entity.permissions()?,
            session_user_id: entity.is_user().then(|| entity.user_id()).flatten(),
            user_id: entity.user_id()?,
        })
    }
}

/// What to search for. Text matches an entity's [`search_predicates`]; every tag must be on the
/// entity.
///
/// [`search_predicates`]: crate::server::shared::storage::traits::Storable::search_predicates
#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    text: String,
    tag_ids: Vec<Uuid>,
    pub limit: u32,
}

impl SearchQuery {
    pub fn new(text: &str, tag_ids: Vec<Uuid>, limit: u32) -> Self {
        Self {
            text: text.trim().to_string(),
            tag_ids,
            limit,
        }
    }

    /// Whether there is anything to search for.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty() && self.tag_ids.is_empty()
    }

    /// `filter` narrowed to live rows matching the text and carrying every tag, or `None` when `T`
    /// can't match: it defines no search predicates, or tags were asked for and `T` takes none.
    /// The caller applies the limit.
    pub fn narrow<T: Entity>(&self, filter: StorableFilter<T>) -> Option<StorableFilter<T>> {
        if T::search_predicates().is_empty()
            || self.is_empty()
            || (!self.tag_ids.is_empty() && !T::is_taggable())
        {
            return None;
        }
        let mut filter = if T::HAS_SCD2 { filter.live() } else { filter };
        // One condition per tag, so they combine with AND.
        for tag_id in &self.tag_ids {
            filter = filter.has_any_tags(std::slice::from_ref(tag_id), T::entity_type());
        }
        if !self.text.is_empty() {
            filter = filter.text_search(&self.text);
        }
        Some(filter)
    }
}
