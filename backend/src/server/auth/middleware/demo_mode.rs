//! Global middleware that blocks non-GET requests for demo mode organizations.
//!
//! This middleware enforces demo mode restrictions at the request level,
//! blocking all mutating operations (POST, PUT, DELETE, PATCH) for demo
//! organizations except for owners who retain full access.
//!
//! Daemons associated with demo organizations are also blocked from
//! mutating operations since they don't have owner permissions.

use crate::server::{
    auth::middleware::{auth::AuthenticatedEntity, cache::CachedSite},
    billing::types::base::BillingPlan,
    config::AppState,
    shared::types::api::ApiError,
    users::r#impl::permissions::UserOrgPermissions,
};
use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

/// Middleware that blocks non-safe HTTP methods for demo mode organizations.
///
/// Safe methods (GET, HEAD, OPTIONS) are always allowed.
/// For other methods:
/// - Non-demo organizations: allowed
/// - Demo organizations with Owner permissions: allowed
/// - Demo organizations with other permissions (including daemons): blocked
pub async fn demo_mode_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Allow safe methods (GET, HEAD, OPTIONS)
    if request.method().is_safe() {
        return next.run(request).await;
    }

    // Allow auth endpoints (login, logout, register, OIDC callbacks, etc.)
    // These must work regardless of demo mode to allow users to authenticate
    if request.uri().path().starts_with("/api/auth/") {
        return next.run(request).await;
    }

    let (mut parts, body) = request.into_parts();

    let entity = AuthenticatedEntity::from_request_parts(&mut parts, &state)
        .await
        .ok();

    // Get org ID and permissions based on auth type
    let (organization_id, permissions) = match &entity {
        Some(AuthenticatedEntity::User {
            organization_id,
            permissions,
            ..
        }) => (Some(*organization_id), Some(*permissions)),
        Some(AuthenticatedEntity::ApiKey {
            organization_id,
            permissions,
            ..
        }) => (Some(*organization_id), Some(*permissions)),
        Some(AuthenticatedEntity::Daemon { site_id, .. }) => {
            // Daemons: look up org from site
            match CachedSite::get_or_load(&mut parts, &state, site_id).await {
                Ok(site) => (Some(site.base.organization_id), None),
                Err(_) => (None, None),
            }
        }
        // System/Anonymous/ExternalService - allow through
        _ => {
            let request = Request::from_parts(parts, body);
            return next.run(request).await;
        }
    };

    let Some(organization_id) = organization_id else {
        let request = Request::from_parts(parts, body);
        return next.run(request).await;
    };

    // Load organization (uses caching)
    let organization =
        match super::cache::CachedOrganization::get_or_load(&mut parts, &state, &organization_id)
            .await
        {
            Ok(org) => org,
            Err(e) => return e.into_response(),
        };
    let plan = organization
        .base
        .plan
        .unwrap_or_else(crate::server::billing::plans::get_free_plan);

    let request = Request::from_parts(parts, body);

    // Only block demo organizations
    if !matches!(plan, BillingPlan::Demo(_)) {
        return next.run(request).await;
    }

    // Allow owners in demo mode (daemons have no permissions, so they're blocked)
    if permissions == Some(UserOrgPermissions::Owner) {
        return next.run(request).await;
    }

    // Block non-GET for non-owners in demo mode
    ApiError::demo_mode_blocked().into_response()
}
