use crate::server::openapi::tags as api_tags;
use crate::server::{
    auth::middleware::permissions::{Authorized, Viewer},
    config::AppState,
    shared::{
        extractors::Query,
        types::api::{ApiError, ApiResponse, ApiResult},
    },
};

use super::{
    scope::{SearchQuery, SearchScope},
    service::{GLOBAL_SEARCH_FIRST_PAGE, GLOBAL_SEARCH_MAX_PAGE, global_search},
    types::{GlobalSearchQuery, GlobalSearchResponse},
};

use axum::{extract::State, response::Json};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new().routes(routes!(search))
}

/// Search every entity type
///
/// Returns a page of matches of each entity type the caller can list, on the sites and in the
/// organization they can access, with each type's total. Each type matches the text against its
/// own fields; tags narrow every type to entities carrying all of them. Name `entity_type` with
/// `offset` to page through one type.
#[utoipa::path(
    get,
    path = "",
    tags = [api_tags::SEARCH, api_tags::INTERNAL],
    params(GlobalSearchQuery),
    responses(
        (status = 200, description = "Matches grouped by entity type", body = ApiResponse<GlobalSearchResponse>),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn search(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(query): Query<GlobalSearchQuery>,
) -> ApiResult<Json<ApiResponse<GlobalSearchResponse>>> {
    let scope =
        SearchScope::for_entity(&auth.entity).ok_or_else(ApiError::organization_required)?;
    let limit = query
        .limit
        .unwrap_or(GLOBAL_SEARCH_FIRST_PAGE)
        .clamp(1, GLOBAL_SEARCH_MAX_PAGE);
    let search = SearchQuery::new(
        query.q.as_deref().unwrap_or_default(),
        query.tag_ids.unwrap_or_default(),
        limit,
        query.offset.unwrap_or_default(),
    );
    let response = global_search(&state.services, &scope, &search, query.entity_type).await?;
    Ok(Json(ApiResponse::success(response)))
}
