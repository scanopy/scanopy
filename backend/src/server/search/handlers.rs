use crate::server::openapi::tags as api_tags;
use crate::server::{
    auth::middleware::permissions::{Authorized, Viewer},
    config::AppState,
    shared::{
        extractors::Query,
        services::search::{SearchQuery, SearchScope},
        types::api::{ApiError, ApiResponse, ApiResult},
    },
};

use super::{
    service::{GLOBAL_SEARCH_LIMIT, global_search},
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
/// Returns up to five matches of each entity type the caller can list, on the sites and in the
/// organization they can access. Each type matches the text against its own fields; tags narrow
/// every type to entities carrying all of them.
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
    let query = SearchQuery::new(
        query.q.as_deref().unwrap_or_default(),
        query.tag_ids.unwrap_or_default(),
        GLOBAL_SEARCH_LIMIT,
    );
    let response = global_search(&state.services, &scope, &query).await?;
    Ok(Json(ApiResponse::success(response)))
}
