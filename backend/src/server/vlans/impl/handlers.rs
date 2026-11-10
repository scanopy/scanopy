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
    vlans::{
        handlers::{VlanFilterQuery, VlanOrderField},
        r#impl::base::Vlan,
        service::VlanService,
    },
};
use async_trait::async_trait;

/// The links between subnets and the VLANs they sit on.
const SUBNET_VLANS: Junction = Junction {
    table: "subnet_vlans",
    own_column: "vlan_id",
    other_column: "subnet_id",
    has_history: true,
};

impl VlanFilterQuery {
    /// Everything the VLANs list narrows by: access, sites, sources, subnets, tags and search.
    pub fn narrow(&self, scope: &ListScope) -> ApiResult<StorableFilter<Vlan>> {
        let filter = narrow_by_tags_and_search(
            base_list_filter::<Vlan>(scope, self),
            self.tag_ids.as_deref(),
            self.tag_sets.as_deref(),
            self.search.as_deref(),
        )?;
        Ok(match &self.subnet_ids {
            Some(ids) if !ids.is_empty() => filter.linked_to(SUBNET_VLANS, ids),
            _ => filter,
        })
    }
}

#[async_trait]
impl CrudHandlers for Vlan {
    type Service = VlanService;
    type FilterQuery = VlanFilterQuery;
    type OrderField = VlanOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.vlan_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(Self::OrderField::VlanNumber)
    }

    /// Shared by the list and the CSV export, so an export returns the rows the list showed.
    async fn list_filter(
        _state: &AppState,
        query: &VlanFilterQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<Vlan>> {
        query.narrow(scope)
    }
}

impl ListRows for Vlan {
    type Item = Vlan;
    type Field = VlanOrderField;
    type Query = VlanFilterQuery;

    fn ordering(query: &VlanFilterQuery) -> ListOrdering<VlanOrderField> {
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
impl PaginatedList for Vlan {
    fn default_order() -> String {
        "vlans.vlan_number ASC".to_string()
    }

    async fn items(
        _state: &AppState,
        rows: Vec<Vlan>,
        _query: &VlanFilterQuery,
    ) -> ApiResult<Vec<Vlan>> {
        Ok(rows)
    }
}
