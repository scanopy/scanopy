use crate::server::{
    config::AppState,
    daemon_api_keys::{
        handlers::{DaemonApiKeyFilterQuery, DaemonApiKeyOrderField},
        r#impl::base::DaemonApiKey,
        service::DaemonApiKeyService,
    },
    shared::{
        handlers::{
            list::{ListOrdering, ListRows, PaginatedList},
            ordering::TagGroupOrdering,
            query::narrow_by_tags_and_search,
            traits::{CrudHandlers, ListScope, base_list_filter},
        },
        storage::filter::StorableFilter,
        types::api::ApiResult,
    },
};
use async_trait::async_trait;

impl DaemonApiKeyFilterQuery {
    /// Everything the daemon API keys list narrows by: access, sites, binding, enabled state,
    /// tags and search.
    pub fn narrow(&self, scope: &ListScope) -> ApiResult<StorableFilter<DaemonApiKey>> {
        narrow_by_tags_and_search(
            base_list_filter::<DaemonApiKey>(scope, self),
            self.tag_ids.as_deref(),
            self.tag_sets.as_deref(),
            self.search.as_deref(),
        )
    }
}

#[async_trait]
impl CrudHandlers for DaemonApiKey {
    type Service = DaemonApiKeyService;
    type FilterQuery = DaemonApiKeyFilterQuery;
    type OrderField = DaemonApiKeyOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.daemon_api_key_service
    }

    async fn list_filter(
        _state: &AppState,
        query: &DaemonApiKeyFilterQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<DaemonApiKey>> {
        query.narrow(scope)
    }
}

impl ListRows for DaemonApiKey {
    type Item = DaemonApiKey;
    type Field = DaemonApiKeyOrderField;
    type Query = DaemonApiKeyFilterQuery;

    fn ordering(query: &DaemonApiKeyFilterQuery) -> ListOrdering<DaemonApiKeyOrderField> {
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
impl PaginatedList for DaemonApiKey {
    async fn items(
        _state: &AppState,
        rows: Vec<DaemonApiKey>,
        _query: &DaemonApiKeyFilterQuery,
    ) -> ApiResult<Vec<DaemonApiKey>> {
        Ok(rows)
    }
}
