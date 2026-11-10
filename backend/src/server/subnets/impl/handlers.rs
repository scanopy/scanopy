use std::collections::HashMap;

use crate::server::{
    config::AppState,
    shared::{
        handlers::{
            list::{Computed, ComputedValue, ListOrdering, ListRows, PaginatedList},
            ordering::{OrderField, TagGroupOrdering},
            query::narrow_by_tags_and_search,
            traits::{CrudHandlers, ListScope, base_list_filter},
            tree::TreePosition,
        },
        storage::filter::StorableFilter,
        types::api::ApiResult,
    },
    subnets::{
        handlers::{SubnetFilterQuery, SubnetOrderField},
        r#impl::{base::Subnet, nesting::SubnetResponse},
        service::SubnetService,
    },
};
use async_trait::async_trait;
use uuid::Uuid;

impl SubnetFilterQuery {
    /// Everything the subnets list narrows by except staleness, which needs each site's cutoff:
    /// access, sites, types, sources, runtimes, user-managed ranges, tags and search.
    pub fn narrow(&self, scope: &ListScope) -> ApiResult<StorableFilter<Subnet>> {
        narrow_by_tags_and_search(
            base_list_filter::<Subnet>(scope, self),
            self.tag_ids.as_deref(),
            self.tag_sets.as_deref(),
            self.search.as_deref(),
        )
    }
}

#[async_trait]
impl CrudHandlers for Subnet {
    type Service = SubnetService;
    type FilterQuery = SubnetFilterQuery;
    type OrderField = SubnetOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.subnet_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::Name)
    }

    /// Shared by the list and the CSV export, so an export returns the rows the list showed.
    async fn list_filter(
        state: &AppState,
        query: &SubnetFilterQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<Subnet>> {
        let filter = query.narrow(scope)?;
        // Staleness is per-site, so resolve each accessible site's cutoff and let the filter
        // compare every row against its own.
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

impl ListRows for Subnet {
    type Item = SubnetResponse;
    type Field = SubnetOrderField;
    type Query = SubnetFilterQuery;

    fn ordering(query: &SubnetFilterQuery) -> ListOrdering<SubnetOrderField> {
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

    fn is_computed(field: &SubnetOrderField) -> bool {
        matches!(field, SubnetOrderField::Utilization)
    }

    fn computed(field: &SubnetOrderField, item: &SubnetResponse) -> Option<Computed> {
        match field {
            SubnetOrderField::Utilization => Some(Computed {
                value: (item.usable_addresses > 0).then(|| {
                    ComputedValue::Number(item.used_addresses as f64 / item.usable_addresses as f64)
                }),
                group: None,
            }),
            _ => None,
        }
    }
}

#[async_trait]
impl PaginatedList for Subnet {
    /// Address order, which is also the order a range sits before the ranges inside it.
    fn default_order() -> String {
        format!("{} ASC, subnets.id ASC", SubnetOrderField::Cidr.to_sql())
    }

    /// Each subnet with its usage and its place among its site's ranges.
    async fn items(
        state: &AppState,
        rows: Vec<Subnet>,
        query: &SubnetFilterQuery,
    ) -> ApiResult<Vec<SubnetResponse>> {
        Ok(state
            .services
            .subnet_service
            .with_usage(rows, query.at)
            .await?)
    }

    fn groups_as_tree(field: &SubnetOrderField) -> bool {
        matches!(field, SubnetOrderField::Cidr)
    }

    async fn tree_positions(
        state: &AppState,
        ids: &[Uuid],
    ) -> ApiResult<HashMap<Uuid, TreePosition>> {
        Ok(state.services.subnet_service.tree_positions_of(ids).await?)
    }
}
