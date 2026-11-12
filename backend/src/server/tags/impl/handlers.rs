use crate::server::{
    config::AppState,
    shared::{
        handlers::{
            list::{ListOrdering, ListRows, PaginatedList},
            query::narrow_by_tags_and_search,
            traits::{CrudHandlers, ListScope, base_list_filter},
        },
        storage::filter::StorableFilter,
        types::api::ApiResult,
    },
    tags::{
        handlers::{TagFilterQuery, TagOrderField},
        r#impl::base::Tag,
        service::TagService,
    },
};
use async_trait::async_trait;

impl TagFilterQuery {
    /// Everything the tags list narrows by: the organization, colour, group, icon and search.
    pub fn narrow(&self, scope: &ListScope) -> ApiResult<StorableFilter<Tag>> {
        narrow_by_tags_and_search(
            base_list_filter::<Tag>(scope, self),
            None,
            None,
            self.search.as_deref(),
        )
    }
}

#[async_trait]
impl CrudHandlers for Tag {
    type Service = TagService;
    type FilterQuery = TagFilterQuery;
    type OrderField = TagOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.tag_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::Name)
    }

    async fn list_filter(
        _state: &AppState,
        query: &TagFilterQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<Tag>> {
        query.narrow(scope)
    }
}

impl ListRows for Tag {
    type Item = Tag;
    type Field = TagOrderField;
    type Query = TagFilterQuery;

    fn ordering(query: &TagFilterQuery) -> ListOrdering<TagOrderField> {
        ListOrdering {
            group_by: query.group_by,
            order_by: query.order_by,
            direction: query.order_direction,
            tag_group: Default::default(),
        }
    }
}

#[async_trait]
impl PaginatedList for Tag {
    async fn items(
        _state: &AppState,
        rows: Vec<Tag>,
        _query: &TagFilterQuery,
    ) -> ApiResult<Vec<Tag>> {
        Ok(rows)
    }
}
