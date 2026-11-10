use crate::server::{
    config::AppState,
    daemons::{
        handlers::{DaemonFilterQuery, DaemonOrderField},
        r#impl::{
            api::{DaemonHealth, DaemonResponse},
            base::Daemon,
            version::DaemonVersionPolicy,
        },
        service::DaemonService,
    },
    shared::{
        handlers::{
            list::{Computed, ComputedValue, ListOrdering, ListRows, PaginatedList},
            ordering::TagGroupOrdering,
            traits::{CrudHandlers, ListScope},
        },
        storage::filter::StorableFilter,
        types::api::ApiResult,
    },
};
use async_trait::async_trait;
use strum::IntoEnumIterator;
use uuid::Uuid;

#[async_trait]
impl CrudHandlers for Daemon {
    type Service = DaemonService;
    type FilterQuery = DaemonFilterQuery;
    type OrderField = DaemonOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.daemon_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::Name)
    }

    async fn list_filter(
        _state: &AppState,
        query: &DaemonFilterQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<Daemon>> {
        query.narrow(scope)
    }
}

impl ListRows for Daemon {
    type Item = DaemonResponse;
    type Field = DaemonOrderField;
    type Query = DaemonFilterQuery;

    fn ordering(query: &DaemonFilterQuery) -> ListOrdering<DaemonOrderField> {
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

    fn is_computed(field: &DaemonOrderField) -> bool {
        matches!(field, DaemonOrderField::Status | DaemonOrderField::Version)
    }

    fn computed(field: &DaemonOrderField, item: &DaemonResponse) -> Option<Computed> {
        match field {
            // Most severe first: the order the statuses are declared in.
            DaemonOrderField::Status => Some(Computed {
                value: DaemonHealth::iter()
                    .position(|health| health == item.health)
                    .map(|rank| ComputedValue::Number(rank as f64)),
                group: Some(<&str>::from(item.health).to_string()),
            }),
            DaemonOrderField::Version => Some(Computed {
                value: item.base.version.as_ref().map(|v| {
                    ComputedValue::Number(
                        v.major as f64 * 1e12 + v.minor as f64 * 1e6 + v.patch as f64,
                    )
                }),
                group: item.base.version.as_ref().map(ToString::to_string),
            }),
            _ => None,
        }
    }

    fn keep(query: &DaemonFilterQuery, item: &DaemonResponse) -> bool {
        query
            .statuses
            .as_ref()
            .is_none_or(|statuses| statuses.is_empty() || statuses.contains(&item.health))
    }
}

#[async_trait]
impl PaginatedList for Daemon {
    /// Each daemon with its version status, health and interfaced subnets.
    async fn items(
        state: &AppState,
        rows: Vec<Daemon>,
        _query: &DaemonFilterQuery,
    ) -> ApiResult<Vec<DaemonResponse>> {
        let ids: Vec<Uuid> = rows.iter().map(|d| d.id).collect();
        let mut subnets = state
            .services
            .daemon_service
            .get_interfaced_subnet_ids_batch(&ids)
            .await;
        let policy = DaemonVersionPolicy::default();
        Ok(rows
            .into_iter()
            .map(|daemon| {
                let version_status = policy.evaluate(daemon.base.version.as_ref());
                let interfaced = subnets.remove(&daemon.id).unwrap_or_default();
                DaemonResponse::new(daemon, version_status, interfaced)
            })
            .collect())
    }

    fn filters_in_memory(query: &DaemonFilterQuery) -> bool {
        query.statuses.as_ref().is_some_and(|s| !s.is_empty())
    }
}
