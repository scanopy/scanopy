use std::collections::HashMap;

use crate::server::{
    config::AppState,
    hosts::{
        handlers::{HostFilterQuery, HostOrderField},
        r#impl::{api::HostResponse, base::Host},
        service::HostService,
    },
    shared::{
        handlers::{
            list::{ListOrdering, ListRows, PaginatedList},
            ordering::TagGroupOrdering,
            traits::{CrudHandlers, ListScope},
            tree::TreePosition,
        },
        storage::filter::StorableFilter,
        types::api::ApiResult,
    },
};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
impl CrudHandlers for Host {
    type Service = HostService;
    type FilterQuery = HostFilterQuery;
    type OrderField = crate::server::hosts::handlers::HostOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.host_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::Name)
    }

    /// Everything the Hosts list narrows by, shared by the list and both exports so an export
    /// returns exactly the rows the list showed for the same query.
    async fn list_filter(
        state: &AppState,
        query: &HostFilterQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<Host>> {
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

impl ListRows for Host {
    type Item = HostResponse;
    type Field = HostOrderField;
    type Query = HostFilterQuery;

    fn ordering(query: &HostFilterQuery) -> ListOrdering<HostOrderField> {
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
impl PaginatedList for Host {
    /// Each host with its children, tags, tree placement and credential assignments.
    async fn items(
        state: &AppState,
        rows: Vec<Host>,
        query: &HostFilterQuery,
    ) -> ApiResult<Vec<HostResponse>> {
        let mut items = state
            .services
            .host_service
            .host_responses(rows, query.at, query.include_children.unwrap_or(true))
            .await?;

        let host_ids: Vec<Uuid> = items.iter().map(|h| h.id).collect();
        let cred_map = state
            .services
            .credential_service
            .get_credential_assignments_for_hosts(&host_ids)
            .await?;
        for host in &mut items {
            if let Some(assignments) = cred_map.get(&host.id) {
                host.credential_assignments = assignments.clone();
            }
        }
        Ok(items)
    }

    /// "Virtualized By" groups each host under the top of its virtualization chain, while sorting
    /// and filter options stay on the immediate service's name.
    fn groups_as_tree(field: &HostOrderField) -> bool {
        matches!(field, HostOrderField::VirtualizedBy)
    }

    async fn tree_positions(
        state: &AppState,
        ids: &[Uuid],
    ) -> ApiResult<HashMap<Uuid, TreePosition>> {
        Ok(state.services.host_service.tree_positions_of(ids).await?)
    }
}
