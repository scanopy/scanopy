use crate::server::{
    config::AppState,
    discovery::handlers::{DiscoveryFilterQuery, DiscoveryOrderField},
    discovery::{r#impl::base::Discovery, service::DiscoveryService},
    shared::{
        handlers::{
            list::{ListOrdering, ListRows, PaginatedList},
            ordering::{OrderField, TagGroupOrdering},
            query::narrow_by_tags_and_search,
            traits::{CrudHandlers, ListScope, base_list_filter},
        },
        storage::filter::StorableFilter,
        types::api::ApiResult,
    },
};
use async_trait::async_trait;

impl DiscoveryFilterQuery {
    /// Everything the discovery lists narrow by: access, the query's own fields, how a
    /// configuration runs, legacy types, forced full scans, tags and search.
    pub fn narrow(&self, scope: &ListScope) -> ApiResult<StorableFilter<Discovery>> {
        let filter = narrow_by_tags_and_search(
            base_list_filter::<Discovery>(scope, self),
            self.tag_ids.as_deref(),
            self.tag_sets.as_deref(),
            self.search.as_deref(),
        )?;
        let filter = match &self.run_types {
            Some(types) if !types.is_empty() => filter.json_text_in("run_type", &["type"], types),
            _ => filter,
        };
        let filter = match &self.legacy {
            Some(values) => filter.expr_text_in(
                DiscoveryOrderField::Legacy.to_sql(),
                &values.iter().map(bool::to_string).collect::<Vec<_>>(),
            ),
            None => filter,
        };
        Ok(match &self.force_full_scan {
            Some(values) => filter.bool_column_in("force_full_scan", values),
            None => filter,
        })
    }
}

#[async_trait]
impl CrudHandlers for Discovery {
    type Service = DiscoveryService;
    type FilterQuery = DiscoveryFilterQuery;
    type OrderField = crate::server::discovery::handlers::DiscoveryOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.discovery_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::Name)
    }

    /// Shared by the list and the CSV export, so an export returns the rows the list showed.
    async fn list_filter(
        _state: &AppState,
        query: &DiscoveryFilterQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<Discovery>> {
        query.narrow(scope)
    }
}

impl ListRows for Discovery {
    type Item = Discovery;
    type Field = crate::server::discovery::handlers::DiscoveryOrderField;
    type Query = DiscoveryFilterQuery;

    fn ordering(query: &DiscoveryFilterQuery) -> ListOrdering<Self::Field> {
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
impl PaginatedList for Discovery {
    fn default_order() -> String {
        "discovery.created_at DESC".to_string()
    }

    async fn items(
        _state: &AppState,
        rows: Vec<Discovery>,
        _query: &DiscoveryFilterQuery,
    ) -> ApiResult<Vec<Discovery>> {
        Ok(rows)
    }
}
