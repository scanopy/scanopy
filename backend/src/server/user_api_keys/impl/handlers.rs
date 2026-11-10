use crate::server::{
    config::AppState,
    shared::{
        handlers::{
            list::{ListOrdering, ListRows, PaginatedList},
            ordering::TagGroupOrdering,
            query::narrow_by_tags_and_search,
            traits::{CrudHandlers, ListScope, base_list_filter},
        },
        storage::filter::{Junction, StorableFilter},
        types::api::ApiResult,
    },
    user_api_keys::{
        handlers::{UserApiKeyFilterQuery, UserApiKeyOrderField},
        r#impl::base::UserApiKey,
        service::UserApiKeyService,
    },
};
use async_trait::async_trait;

/// The sites each user API key may reach. A key's All sites row names no site, so it matches none.
const KEY_SITES: Junction = Junction {
    table: "site_assignments",
    own_column: "api_key_id",
    other_column: "site_id",
    has_history: false,
};

impl UserApiKeyFilterQuery {
    /// Everything the user API keys list narrows by: the caller's own keys, roles, sites,
    /// enabled state, tags and search.
    pub fn narrow(&self, scope: &ListScope) -> ApiResult<StorableFilter<UserApiKey>> {
        let filter = narrow_by_tags_and_search(
            base_list_filter::<UserApiKey>(scope, self),
            self.tag_ids.as_deref(),
            self.tag_sets.as_deref(),
            self.search.as_deref(),
        )?;
        Ok(match &self.site_ids {
            Some(ids) if !ids.is_empty() => filter.linked_to(KEY_SITES, ids),
            _ => filter,
        })
    }
}

#[async_trait]
impl CrudHandlers for UserApiKey {
    type Service = UserApiKeyService;
    type FilterQuery = UserApiKeyFilterQuery;
    type OrderField = UserApiKeyOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.user_api_key_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::Name)
    }

    /// A user API key is listed only to the user it belongs to. A caller with no user behind it
    /// lists none.
    fn access_filter(scope: &ListScope) -> StorableFilter<Self> {
        match scope.user_id {
            Some(user_id) => StorableFilter::new_from_user_id(&user_id),
            None => StorableFilter::new_from_entity_ids(&[]),
        }
    }

    async fn list_filter(
        _state: &AppState,
        query: &UserApiKeyFilterQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<UserApiKey>> {
        query.narrow(scope)
    }
}

impl ListRows for UserApiKey {
    type Item = UserApiKey;
    type Field = UserApiKeyOrderField;
    type Query = UserApiKeyFilterQuery;

    fn ordering(query: &UserApiKeyFilterQuery) -> ListOrdering<UserApiKeyOrderField> {
        ListOrdering {
            group_by: query.group_by,
            order_by: query.order_by,
            direction: query.order_direction,
            tag_group: TagGroupOrdering {
                group_by: query.group_by_tag_group,
                order_by: query.order_by_tag_group,
            },
        }
    }
}

#[async_trait]
impl PaginatedList for UserApiKey {
    /// Each key with the sites it may reach.
    async fn items(
        state: &AppState,
        mut rows: Vec<UserApiKey>,
        _query: &UserApiKeyFilterQuery,
    ) -> ApiResult<Vec<UserApiKey>> {
        state
            .services
            .user_api_key_service
            .hydrate_site_ids(&mut rows)
            .await?;
        Ok(rows)
    }
}
