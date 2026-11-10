use crate::server::{
    config::AppState,
    services::{handlers::ServiceFilterQuery, r#impl::base::Service, service::ServiceService},
    shared::{
        handlers::{
            list::{ListOrdering, ListRows, PaginatedList},
            ordering::TagGroupOrdering,
            traits::{CrudHandlers, ListScope},
        },
        storage::filter::StorableFilter,
        types::api::ApiResult,
    },
};
use async_trait::async_trait;

#[async_trait]
impl CrudHandlers for Service {
    type Service = ServiceService;
    type FilterQuery = ServiceFilterQuery;
    type OrderField = crate::server::services::handlers::ServiceOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.service_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::Name)
    }

    /// Everything the Services list narrows by, shared by the list and the CSV export so an
    /// export returns exactly the rows the list showed for the same query.
    async fn list_filter(
        state: &AppState,
        query: &ServiceFilterQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<Service>> {
        let filter = query.narrow(&scope.site_ids, scope.organization_id)?;

        // Staleness is per-site, so resolve each accessible site's cutoff and
        // let the filter compare every row against its own.
        Ok(match query.stale {
            Some(stale) => {
                let cutoffs = state
                    .services
                    .site_service
                    .stale_cutoffs(&scope.site_ids)
                    .await?;
                filter.stale_by_site(&cutoffs, stale)
            }
            None => filter,
        })
    }
}

impl ListRows for Service {
    type Item = Service;
    type Field = crate::server::services::handlers::ServiceOrderField;
    type Query = ServiceFilterQuery;

    fn ordering(query: &ServiceFilterQuery) -> ListOrdering<Self::Field> {
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
impl PaginatedList for Service {
    async fn items(
        _state: &AppState,
        rows: Vec<Service>,
        _query: &ServiceFilterQuery,
    ) -> ApiResult<Vec<Service>> {
        Ok(rows)
    }
}
