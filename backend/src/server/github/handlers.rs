use axum::{Extension, Json};
use reqwest;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::server::shared::{
    handlers::cache::AppCache,
    types::api::{ApiError, ApiResponse, ApiResult},
};

#[derive(Serialize, Deserialize)]
struct GitHubRepoResponse {
    stargazers_count: u32,
}

const CACHE_KEY: &str = "github_stars";

/// Get GitHub star count
///
/// Returns the current star count for the Scanopy GitHub repository.
#[utoipa::path(
    get,
    path = "/api/github-stars",
    tags = ["github", "internal"],
    responses(
        (status = 200, description = "GitHub star count", body = ApiResponse<u32>)
    )
)]
pub async fn get_stars(
    Extension(cache): Extension<Arc<AppCache>>,
) -> ApiResult<Json<ApiResponse<u32>>> {
    // Check cache first
    if let Some(cached) = cache.get::<u32>(CACHE_KEY).await {
        return Ok(Json(ApiResponse::success(cached)));
    }

    // Cache miss - fetch from GitHub
    let client = reqwest::Client::new();
    let request = client
        .get("https://api.github.com/repos/scanopy/scanopy")
        .header("User-Agent", "Scanopy");

    match request.send().await {
        Ok(response) => {
            if !response.status().is_success() {
                tracing::error!("GitHub API error: {}", response.status());
                return Err(ApiError::bad_gateway(
                    "Failed to fetch data from GitHub".to_string(),
                ));
            }

            match response.json::<GitHubRepoResponse>().await {
                Ok(data) => {
                    // Cache for 6 hours
                    cache.set(CACHE_KEY, data.stargazers_count, 168).await;

                    Ok(Json(ApiResponse::success(data.stargazers_count)))
                }
                Err(e) => {
                    tracing::error!("Failed to parse GitHub response: {}", e);
                    Err(ApiError::bad_gateway(
                        "Failed to process GitHub response".to_string(),
                    ))
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to fetch from GitHub: {}", e);
            Err(ApiError::bad_gateway(
                "GitHub service unavailable".to_string(),
            ))
        }
    }
}
