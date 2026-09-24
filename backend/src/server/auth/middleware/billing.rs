//! Billing middleware that checks subscription status for authenticated requests.
//!
//! This middleware examines each request and:
//! - For authenticated requests (user, API key, daemon): Checks billing status, returns 402 if not active
//! - For unauthenticated requests: Passes through (handler auth will reject if needed)
//!
//! Exemptions (always allowed regardless of billing):
//! - Community, Free, CommercialSelfHosted, and Demo plans
//! - Self-hosted instances (no stripe_secret configured)
//!
//! Orgs on a self-hosted license plan (SelfHostedStandard / SelfHostedPlus)
//! run Scanopy on their own servers, so on the cloud they only reach the
//! routes the Settings modal needs; everything else returns 403.
//!
//! A lapsed org (subscription ended, no paid plan chosen since) keeps its
//! plan and can read everything, but every mutating request outside the
//! Settings routes returns 402 until it chooses a paid plan.

use crate::server::{
    auth::middleware::{auth::AuthenticatedEntity, cache::CachedNetwork},
    billing::types::base::BillingPlan,
    config::AppState,
    shared::types::api::ApiError,
};
use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

/// Middleware that enforces billing requirements for authenticated requests.
///
/// Apply this middleware at the router level to groups of routes that require billing.
/// Checks billing for users (session), user API keys, and daemons.
/// Caches looked-up entities in request extensions for subsequent handlers.
pub async fn require_billing_for_users(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Check if billing is enabled:
    // - Production: enabled when stripe_secret is configured
    // - Testing: enabled when enforce_billing_for_testing is true
    let billing_enabled =
        state.config.stripe_secret.is_some() || state.config.enforce_billing_for_testing;
    if !billing_enabled {
        return next.run(request).await;
    }

    // Split request to access parts for caching
    let (mut parts, body) = request.into_parts();

    // Extract authenticated entity (cached in extensions)
    let entity = AuthenticatedEntity::from_request_parts(&mut parts, &state)
        .await
        .ok();

    // Get organization ID based on auth type, caching network lookup for daemons
    let organization_id: Option<Uuid> = match &entity {
        Some(AuthenticatedEntity::User {
            organization_id, ..
        }) => Some(*organization_id),
        Some(AuthenticatedEntity::ApiKey {
            organization_id, ..
        }) => Some(*organization_id),
        Some(AuthenticatedEntity::Daemon { network_id, .. }) => {
            // Use cached network lookup
            match CachedNetwork::get_or_load(&mut parts, &state, network_id).await {
                Ok(network) => Some(network.base.organization_id),
                Err(_) => None,
            }
        }
        _ => None,
    };

    let Some(organization_id) = organization_id else {
        // Not authenticated or no org - pass through (handler auth will reject if needed)
        let request = Request::from_parts(parts, body);
        return next.run(request).await;
    };

    // Cached organization for the request — used by both this middleware
    // and any downstream feature checks.
    let organization =
        match super::cache::CachedOrganization::get_or_load(&mut parts, &state, &organization_id)
            .await
        {
            Ok(org) => org,
            Err(e) => return e.into_response(),
        };

    // Reassemble request with cached entities in extensions
    let request = Request::from_parts(parts, body);

    // Check plan type - some plans are exempt from billing checks.
    // `unwrap_or_default()` treats `plan = None` (newly-registered orgs that
    // haven't selected a plan yet) as the build's default plan
    // (Community / CommercialSelfHosted), which matches the exempt arm and
    // lets `/api/v1/organizations` succeed. The frontend reads the resulting
    // `plan = null` and opens BillingPlanModal to force plan selection.
    let plan = organization.base.plan.unwrap_or_default();
    if plan.license_plan().is_some() {
        if settings_route(request.method(), request.uri().path()) {
            return next.run(request).await;
        }
        return ApiError::self_hosted_plan_locked().into_response();
    }
    if matches!(
        plan,
        BillingPlan::Community(_)
            | BillingPlan::Free(_)
            | BillingPlan::CommercialSelfHosted(_)
            | BillingPlan::Demo(_)
    ) {
        return next.run(request).await;
    }

    // Check subscription status. None = no subscription yet (must select a
    // plan); Cancelled = the subscription ended and the org kept its plan, so
    // it is read-only until it chooses a paid plan; everything else allows
    // the request through (Active / Trialing / PendingCancellation / PastDue
    // / Paused all keep features available — Paused is a Stripe collection
    // pause, not a feature lockout).
    if organization.is_lapsed() {
        if request.method().is_safe() || settings_route(request.method(), request.uri().path()) {
            return next.run(request).await;
        }
        return ApiError::billing_plan_lapsed().into_response();
    }
    match organization.base.plan_status {
        Some(_) => next.run(request).await,
        None => billing_error_response("Active billing plan required. Please select a plan."),
    }
}

/// Billed routes the Settings modal calls, which stay open to an org on a
/// self-hosted license plan and to a lapsed org: reading, renaming, and
/// deleting the org, and editing the current user. Everything else Settings
/// needs (auth, billing, licenses, config) sits outside this middleware
/// already.
fn settings_route(method: &Method, path: &str) -> bool {
    const ORGANIZATIONS: &str = "/api/v1/organizations";
    const USERS: &str = "/api/v1/users";
    let single_entity = |prefix: &str| {
        path.strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix('/'))
            .is_some_and(|id| !id.is_empty() && !id.contains('/'))
    };

    match *method {
        Method::GET => path == ORGANIZATIONS,
        Method::PUT => single_entity(ORGANIZATIONS) || single_entity(USERS),
        Method::DELETE => single_entity(ORGANIZATIONS),
        _ => false,
    }
}

fn billing_error_response(message: &str) -> Response {
    (
        StatusCode::PAYMENT_REQUIRED,
        axum::Json(serde_json::json!({
            "success": false,
            "error": {
                "code": 402,
                "message": message
            }
        })),
    )
        .into_response()
}
