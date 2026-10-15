//! Global middleware that refuses a user session until the user has accepted
//! the current terms (see [`crate::server::auth::terms_gate`]).
//!
//! Only user sessions are gated. API keys, daemons and external services
//! pass through, and so do the routes the gate itself needs: auth (current
//! user, accept, logout), config, health, version, and the org read the app
//! shell makes on every page.

use crate::server::{
    auth::{middleware::auth::AuthenticatedEntity, terms_gate},
    config::{AppState, get_deployment_type},
    shared::types::api::ApiError,
};
use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::{Method, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

/// Requests a user who still has to accept the terms can make.
fn allowed_while_gated(method: &Method, path: &str) -> bool {
    !path.starts_with("/api/")
        || path.starts_with("/api/auth/")
        || path == "/api/config"
        || path == "/api/health"
        || path == "/api/version"
        || (method == Method::GET && path == "/api/v1/organizations")
}

pub async fn terms_gate_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let deployment_type = get_deployment_type(&state.config);
    if !deployment_type.requires_terms_acceptance()
        || allowed_while_gated(request.method(), request.uri().path())
    {
        return next.run(request).await;
    }

    let (mut parts, body) = request.into_parts();

    let Ok(AuthenticatedEntity::User {
        organization_id,
        terms_accepted_at,
        ..
    }) = AuthenticatedEntity::from_request_parts(&mut parts, &state).await
    else {
        return next.run(Request::from_parts(parts, body)).await;
    };

    let organization =
        match super::cache::CachedOrganization::get_or_load(&mut parts, &state, &organization_id)
            .await
        {
            Ok(org) => org,
            Err(e) => return e.into_response(),
        };

    if terms_gate::acceptance_required(
        deployment_type,
        organization.base.plan.as_ref(),
        terms_accepted_at,
    ) {
        return ApiError::terms_acceptance_required().into_response();
    }

    next.run(Request::from_parts(parts, body)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gate_leaves_open_only_what_accepting_needs() {
        assert!(allowed_while_gated(&Method::POST, "/api/auth/me"));
        assert!(allowed_while_gated(&Method::POST, "/api/auth/accept-terms"));
        assert!(allowed_while_gated(&Method::POST, "/api/auth/logout"));
        assert!(allowed_while_gated(&Method::GET, "/api/config"));
        assert!(allowed_while_gated(&Method::GET, "/api/v1/organizations"));
        assert!(allowed_while_gated(&Method::GET, "/index.html"));

        assert!(!allowed_while_gated(&Method::GET, "/api/v1/hosts"));
        assert!(!allowed_while_gated(&Method::GET, "/api/v1/shares"));
        assert!(!allowed_while_gated(&Method::POST, "/api/billing/checkout"));
        let org = format!("/api/v1/organizations/{}", uuid::Uuid::nil());
        assert!(!allowed_while_gated(&Method::PUT, &org));
    }
}
