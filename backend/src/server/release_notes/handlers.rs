use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use chrono::NaiveDate;
use semver::Version;
use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::server::{
    auth::middleware::permissions::{Authorized, IsUser},
    config::AppState,
    openapi::tags as api_tags,
    release_notes::catalog::{CHANGELOG_URL, ReleaseNote},
    shared::{
        services::traits::CrudService,
        types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse},
    },
    users::r#impl::base::User,
};

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_unseen_release_notes))
        .routes(routes!(mark_release_notes_seen))
}

/// One release's notes.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReleaseNoteResponse {
    /// The release, as semver (e.g. `0.18.0`).
    #[schema(value_type = String, example = "0.18.0")]
    pub version: Version,
    /// The day the release was prepared.
    pub date: NaiveDate,
    /// The release's headline.
    pub title: String,
    /// The notes, as HTML rendered from the reviewed markdown, without raw HTML.
    pub html: String,
    /// This release on the public changelog.
    pub url: String,
    /// Whether the modal tells the user to upgrade their daemons to this release.
    pub daemon_upgrade_notice: bool,
}

impl From<&ReleaseNote> for ReleaseNoteResponse {
    fn from(note: &ReleaseNote) -> Self {
        Self {
            version: note.version.clone(),
            date: note.date,
            title: note.title.clone(),
            html: note.html.clone(),
            url: note.url(),
            daemon_upgrade_notice: note.daemon_upgrade_notice,
        }
    }
}

/// The releases the caller hasn't dismissed.
#[derive(Debug, Serialize, ToSchema)]
pub struct UnseenReleaseNotes {
    /// Newest first. Empty when there is nothing new.
    pub releases: Vec<ReleaseNoteResponse>,
    /// The full changelog, present when older unseen releases were left out.
    pub changelog_url: Option<String>,
    /// Whether the modal opens on its own: true when any unseen release asks
    /// for it, including ones left out for the changelog.
    pub auto_open: bool,
}

async fn current_user(state: &AppState, auth: &Authorized<IsUser>) -> ApiResult<User> {
    let user_id = auth.user_id().ok_or_else(ApiError::user_required)?;
    state
        .services
        .user_service
        .get_by_id(&user_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<User>(user_id))
}

/// List unseen release notes
///
/// The releases newer than the last one the caller dismissed, up to the
/// version this server runs.
#[utoipa::path(
    get,
    path = "/unseen",
    tags = [api_tags::INTERNAL],
    responses(
        (status = 200, description = "Unseen releases", body = ApiResponse<UnseenReleaseNotes>),
        (status = 401, description = "Not signed in", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn get_unseen_release_notes(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsUser>,
) -> ApiResult<Json<ApiResponse<UnseenReleaseNotes>>> {
    let user = current_user(&state, &auth).await?;
    let unseen = state.services.release_notes_service.unseen(&user);
    Ok(Json(ApiResponse::success(UnseenReleaseNotes {
        releases: unseen.shown.notes.into_iter().map(Into::into).collect(),
        changelog_url: unseen.shown.truncated.then(|| CHANGELOG_URL.to_string()),
        auto_open: unseen.auto_open,
    })))
}

/// Dismiss release notes
///
/// Marks every release up to the version this server runs as seen by the
/// caller, on every device.
#[utoipa::path(
    post,
    path = "/seen",
    tags = [api_tags::INTERNAL],
    responses(
        (status = 200, description = "Dismissed", body = EmptyApiResponse),
        (status = 401, description = "Not signed in", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn mark_release_notes_seen(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsUser>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let user = current_user(&state, &auth).await?;
    state.services.release_notes_service.mark_seen(user).await?;
    Ok(Json(ApiResponse::success(())))
}
