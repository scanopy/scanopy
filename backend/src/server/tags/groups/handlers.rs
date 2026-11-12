use crate::server::auth::middleware::permissions::{Authorized, Can, Read, Write};
use crate::server::openapi::tags as api_tags;
use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::extractors::Query;
use crate::server::shared::handlers::list::{ListOrdering, ListRows, PaginatedList, list_page};
use crate::server::shared::handlers::ordering::{OrderField, in_sql};
use crate::server::shared::handlers::query::{
    FilterQueryExtractor, OrderDirection, PaginationParams, deserialize_bool_values,
    narrow_by_tags_and_search,
};
use crate::server::shared::handlers::traits::{
    CrudHandlers, ListScope, base_list_filter, create_handler,
};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::{Junction, StorableFilter};
use crate::server::shared::storage::traits::{Entity, Storable};
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, PaginatedApiResponse};
use crate::server::tags::groups::{
    base::{TagGroup, TagGroupType},
    service::TagGroupService,
};
use crate::server::tags::r#impl::base::Tag;
use crate::server::{
    config::AppState,
    shared::types::api::{ApiResponse, ApiResult},
};
use async_trait::async_trait;
use axum::extract::Path;
use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

/// Whether a group is one every organization has, from `TagGroupType::is_system`.
pub(crate) static SYSTEM_SQL: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    let names: Vec<String> = TagGroupType::system().map(|t| t.to_string()).collect();
    in_sql("tag_groups.group_type", names.iter().map(String::as_str))
});

/// Fields the tag groups list can be ordered and grouped by.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema, strum::EnumIter,
)]
#[serde(rename_all = "snake_case")]
pub enum TagGroupOrderField {
    #[default]
    CreatedAt,
    Name,
    UpdatedAt,
    /// Whether the group is a system group (`true`) or one the organization named.
    GroupType,
    /// Whether the group keeps its tags in a set order (`true`) or by name.
    Ordered,
}

impl OrderField for TagGroupOrderField {
    fn to_sql(&self) -> &'static str {
        match self {
            Self::CreatedAt => "tag_groups.created_at",
            Self::Name => "lower(tag_groups.name)",
            Self::UpdatedAt => "tag_groups.updated_at",
            Self::GroupType => SYSTEM_SQL.as_str(),
            Self::Ordered => "tag_groups.ordered",
        }
    }
}

/// Query parameters for filtering and ordering tag groups.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct TagGroupFilterQuery {
    /// Only system groups (`true`), only the organization's own (`false`), or both (repeat it).
    /// Sent empty, matches none.
    #[serde(default, deserialize_with = "deserialize_bool_values")]
    pub system: Option<Vec<bool>>,
    /// Only groups keeping their tags in a set order (`true`), by name (`false`), or both (repeat
    /// it). Sent empty, matches none.
    #[serde(default, deserialize_with = "deserialize_bool_values")]
    pub ordered: Option<Vec<bool>>,
    /// Only groups that apply to one of these entity types. Repeat for several.
    pub entity_types: Option<Vec<EntityDiscriminants>>,
    /// `true` also returns groups that apply to every entity type. Set on its own it returns only
    /// those.
    pub include_every_entity_type: Option<bool>,
    /// Only groups holding one of these tags. Repeat for several.
    pub tag_ids: Option<Vec<Uuid>>,
    /// Free-text search. Case-insensitive substring match against the name and description.
    pub search: Option<String>,
    /// Primary ordering field (used for grouping). Always sorts ASC to keep groups together.
    pub group_by: Option<TagGroupOrderField>,
    /// Secondary ordering field (sorting within groups or standalone sort).
    pub order_by: Option<TagGroupOrderField>,
    /// Direction for order_by field (group_by always uses ASC).
    pub order_direction: Option<OrderDirection>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

/// Each tag and the group it belongs to.
const GROUP_TAGS: Junction = Junction {
    table: "tags",
    own_column: "tag_group_id",
    other_column: "id",
    has_history: true,
};

impl TagGroupFilterQuery {
    /// What the tag groups list narrows by in SQL: the organization, system, order, held tags and
    /// search. The entity types a group applies to can be computed in code, so the list filters on
    /// them itself.
    pub fn narrow(&self, scope: &ListScope) -> ApiResult<StorableFilter<TagGroup>> {
        let filter = narrow_by_tags_and_search(
            base_list_filter::<TagGroup>(scope, self),
            None,
            None,
            self.search.as_deref(),
        )?;
        let filter = match &self.system {
            Some(values) => filter.expr_text_in(
                SYSTEM_SQL.as_str(),
                &values.iter().map(bool::to_string).collect::<Vec<_>>(),
            ),
            None => filter,
        };
        let filter = match &self.ordered {
            Some(values) => filter.bool_column_in("ordered", values),
            None => filter,
        };
        Ok(match &self.tag_ids {
            Some(ids) if !ids.is_empty() => filter.linked_to(GROUP_TAGS, ids),
            _ => filter,
        })
    }

    /// Whether `group` applies to an entity type the query asks for.
    fn applies_to_requested(&self, group: &TagGroup) -> bool {
        let every = self.include_every_entity_type.unwrap_or(false);
        let types = self.entity_types.as_deref().unwrap_or_default();
        match &group.base.entity_types {
            None => every,
            Some(applies) => applies.iter().any(|t| types.contains(t)),
        }
    }
}

impl FilterQueryExtractor for TagGroupFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        _user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        filter
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

#[async_trait]
impl CrudHandlers for TagGroup {
    type Service = TagGroupService;
    type FilterQuery = TagGroupFilterQuery;
    type OrderField = TagGroupOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.tag_group_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(TagGroupOrderField::Name)
    }

    async fn list_filter(
        _state: &AppState,
        query: &TagGroupFilterQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<TagGroup>> {
        query.narrow(scope)
    }
}

impl ListRows for TagGroup {
    type Item = TagGroup;
    type Field = TagGroupOrderField;
    type Query = TagGroupFilterQuery;

    fn ordering(query: &TagGroupFilterQuery) -> ListOrdering<TagGroupOrderField> {
        ListOrdering {
            group_by: query.group_by,
            order_by: query.order_by,
            direction: query.order_direction,
            tag_group: Default::default(),
        }
    }

    fn keep(query: &TagGroupFilterQuery, item: &TagGroup) -> bool {
        query.applies_to_requested(item)
    }
}

#[async_trait]
impl PaginatedList for TagGroup {
    /// System groups first, then by type and name: the order every tag group picker uses.
    fn default_order() -> String {
        format!(
            "{} DESC, tag_groups.group_type ASC, lower(tag_groups.name) ASC",
            SYSTEM_SQL.as_str()
        )
    }

    async fn items(
        _state: &AppState,
        rows: Vec<TagGroup>,
        _query: &TagGroupFilterQuery,
    ) -> ApiResult<Vec<TagGroup>> {
        Ok(rows)
    }

    fn filters_in_memory(query: &TagGroupFilterQuery) -> bool {
        query.entity_types.as_ref().is_some_and(|t| !t.is_empty())
            || query.include_every_entity_type.unwrap_or(false)
    }
}

mod generated {
    use super::*;
    crate::crud_get_by_id_handler!(TagGroup);
    crate::crud_update_handler!(TagGroup);
    crate::crud_delete_handler!(TagGroup);
    crate::crud_bulk_delete_handler!(TagGroup);
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_all_tag_groups, create_tag_group))
        .routes(routes!(
            generated::get_by_id,
            generated::update,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
        .routes(routes!(set_tag_order))
}

/// List tag groups
///
/// Returns every tag group in your organization: the system groups (Application, Status) first,
/// then the groups your organization named, by name.
#[utoipa::path(
    get,
    operation_id = "list_tag_groups",
    path = "",
    tag = TagGroup::ENTITY_NAME_PLURAL,
    params(TagGroupFilterQuery),
    responses(
        (status = 200, description = "List of tag groups", body = inline(PaginatedApiResponse<TagGroup>)),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_all_tag_groups(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<TagGroup, Read>>,
    Query(query): Query<TagGroupFilterQuery>,
) -> ApiResult<Json<PaginatedApiResponse<TagGroup>>> {
    // Tag groups are the organization's: `StorableFilter::new_for_access` scopes them to it.
    let scope = ListScope::of(&auth)?;
    Ok(Json(list_page::<TagGroup>(&state, &scope, &query).await?))
}

/// Create tag group
///
/// Creates a group your organization names. Group names are unique within the organization, and
/// "Application" and "Status" are reserved for the system groups.
#[utoipa::path(
    post,
    path = "",
    tag = TagGroup::ENTITY_NAME_PLURAL,
    request_body = TagGroup,
    responses(
        (status = 200, description = "Tag group created", body = ApiResponse<TagGroup>),
        (status = 400, description = "Validation error", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn create_tag_group(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<TagGroup, Write>>,
    ApiJson(group): ApiJson<TagGroup>,
) -> ApiResult<Json<ApiResponse<TagGroup>>> {
    create_handler::<TagGroup>(State(state), auth, ApiJson(group)).await
}

/// Request body for setting a tag group's tag order
#[derive(Debug, Deserialize, ToSchema)]
pub struct TagOrderRequest {
    /// Every tag in the group, each once, in the order to list them.
    pub tag_ids: Vec<Uuid>,
}

/// Reorder tag group tags
///
/// Tables, pickers and the topology list a group's tags in this order.
#[utoipa::path(
    put,
    path = "/{id}/tag-order",
    tags = [TagGroup::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Tag group ID")),
    request_body = TagOrderRequest,
    responses(
        (status = 200, description = "The group's tags, in their new order", body = ApiResponse<Vec<Tag>>),
        (status = 400, description = "The list doesn't name every tag in the group exactly once", body = ApiErrorResponse),
        (status = 404, description = "Tag group not found", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn set_tag_order(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<TagGroup, Write>>,
    Path(id): Path<Uuid>,
    ApiJson(request): ApiJson<TagOrderRequest>,
) -> ApiResult<Json<ApiResponse<Vec<Tag>>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let service = &state.services.tag_group_service;
    let group = service
        .get_by_id(&id)
        .await?
        .filter(|g| g.base.organization_id == organization_id)
        .ok_or_else(|| ApiError::not_found("Tag group not found".to_string()))?;

    let tags = service.reorder_tags(&group, &request.tag_ids).await?;
    Ok(Json(ApiResponse::success(tags)))
}
