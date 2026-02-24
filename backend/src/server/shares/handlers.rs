use std::{num::NonZeroU32, sync::Arc};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DashMapStateStore};
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use axum::http::StatusCode;

use crate::server::{
    auth::{
        middleware::{
            features::{RequireFeature, ShareViewsFeature},
            permissions::{Authorized, Member},
        },
        service::hash_password,
    },
    billing::types::base::BillingPlan,
    config::AppState,
    networks::r#impl::Network,
    organizations::r#impl::base::Organization,
    shared::validation::validate_csp_domain,
    shared::{
        handlers::traits::{CrudHandlers, create_handler, update_handler},
        services::traits::CrudService,
        storage::traits::{Entity, Storage},
        types::{
            api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult},
            error_codes::ErrorCode,
        },
    },
    shares::r#impl::{
        api::{CreateUpdateShareRequest, PublicShareMetadata, ShareWithTopology},
        base::Share,
    },
    topology::types::base::Topology,
};

type ShareRateLimiter = RateLimiter<Uuid, DashMapStateStore<Uuid>, DefaultClock>;

fn share_password_limiter() -> &'static Arc<ShareRateLimiter> {
    static LIMITER: std::sync::OnceLock<Arc<ShareRateLimiter>> = std::sync::OnceLock::new();
    LIMITER.get_or_init(|| {
        Arc::new(RateLimiter::dashmap(
            Quota::with_period(std::time::Duration::from_secs(180))
                .unwrap()
                .allow_burst(NonZeroU32::new(5).unwrap()),
        ))
    })
}

fn check_share_rate_limit(share_id: &Uuid) -> Result<(), ApiError> {
    if share_password_limiter().check_key(share_id).is_err() {
        return Err(ApiError::too_many_requests(
            "Too many password attempts for this share. Please try again later.".to_string(),
        ));
    }
    Ok(())
}

// Generated handlers for generic CRUD operations
mod generated {
    use super::*;
    crate::crud_get_all_handler!(Share);
    crate::crud_get_by_id_handler!(Share);
    crate::crud_delete_handler!(Share);
    crate::crud_bulk_delete_handler!(Share);
    crate::crud_export_csv_handler!(Share);
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ShareQuery {
    #[serde(default)]
    pub embed: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ShareTopologyRequest {
    #[serde(default)]
    pub password: Option<String>,
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        // Authenticated routes
        .routes(routes!(generated::get_all, create_share))
        .routes(routes!(
            generated::get_by_id,
            update_share,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
        .routes(routes!(generated::export_csv))
}

/// Public share routes (no auth required) — mounted separately for permissive CORS
pub fn create_public_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_public_share_metadata))
        .routes(routes!(verify_share_password))
        .route(
            "/public/{id}/topology",
            axum::routing::post(get_share_topology),
        )
}

// ============================================================================
// Authenticated Routes
// ============================================================================

/// Create a new share
#[utoipa::path(
    post,
    path = "",
    tag = Share::ENTITY_NAME_PLURAL,
    request_body = CreateUpdateShareRequest,
    responses(
        (status = 200, description = "Share created", body = ApiResponse<Share>),
        (status = 400, description = "Invalid request", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_share(
    State(state): State<Arc<AppState>>,
    _feature: RequireFeature<ShareViewsFeature>,
    auth: Authorized<Member>,
    Json(CreateUpdateShareRequest {
        mut share,
        password,
    }): Json<CreateUpdateShareRequest>,
) -> ApiResult<Json<ApiResponse<Share>>> {
    // Validate allowed_domains for CSP safety
    if let Some(ref domains) = share.base.allowed_domains {
        for domain in domains {
            validate_csp_domain(domain)?;
        }
    }

    // Hash password if provided
    if let Some(password) = password
        && !password.is_empty()
    {
        share.base.password_hash =
            Some(hash_password(&password).map_err(|e| ApiError::internal_error(&e.to_string()))?);
    }

    share.base.created_by = auth.user_id().ok_or_else(ApiError::user_required)?;

    create_handler::<Share>(State(state), auth, Json(share)).await
}

/// Update a share
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Share::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Share ID")),
    request_body = CreateUpdateShareRequest,
    responses(
        (status = 200, description = "Share updated", body = ApiResponse<Share>),
        (status = 404, description = "Share not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_share(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    Path(id): Path<Uuid>,
    Json(CreateUpdateShareRequest {
        mut share,
        password,
    }): Json<CreateUpdateShareRequest>,
) -> ApiResult<Json<ApiResponse<Share>>> {
    // Validate allowed_domains for CSP safety
    if let Some(ref domains) = share.base.allowed_domains {
        for domain in domains {
            validate_csp_domain(domain)?;
        }
    }

    // Fetch existing to handle password preservation
    let existing = Share::get_service(&state)
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Share>(id))?;

    // Handle password field:
    // - None: preserve existing password_hash
    // - Some(""): remove password (clear password_hash)
    // - Some(value): hash and set new password
    match &password {
        None => {
            // Preserve existing password
            share.base.password_hash = existing.base.password_hash;
        }
        Some(password) if password.is_empty() => {
            // Remove password
            share.base.password_hash = None;
        }
        Some(password) => {
            // Set new password
            share.base.password_hash = Some(
                hash_password(password).map_err(|e| ApiError::internal_error(&e.to_string()))?,
            );
        }
    }

    // Delegate to generic handler
    update_handler::<Share>(State(state), auth, Path(id), Json(share)).await
}

// ============================================================================
// Public Routes (No Authentication Required)
// ============================================================================

/// Helper to get the organization's plan for a share
async fn get_share_org_plan(state: &AppState, share: &Share) -> Result<BillingPlan, ApiError> {
    // Get network to find organization
    let network = state
        .services
        .network_service
        .storage()
        .get_by_id(&share.base.network_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Network>(share.base.network_id))?;

    // Get organization to find plan
    let org = state
        .services
        .organization_service
        .get_by_id(&network.base.organization_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(network.base.organization_id))?;

    Ok(org.base.plan.unwrap_or_default())
}

/// Get share metadata
///
/// Does not include any topology data
#[utoipa::path(
    get,
    path = "/public/{id}",
    tag = Share::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Share ID")),
    responses(
        (status = 200, description = "Share metadata", body = ApiResponse<PublicShareMetadata>),
        (status = 404, description = "Share not found", body = ApiErrorResponse),
    )
)]
async fn get_public_share_metadata(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<PublicShareMetadata>>> {
    let share = state
        .services
        .share_service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Share>(id))?;

    if !share.is_valid() {
        return Err(ApiError::entity_disabled::<Share>());
    }

    Ok(Json(ApiResponse::success(PublicShareMetadata::from(
        &share,
    ))))
}

/// Verify password for a password-protected share (returns success/failure only)
#[utoipa::path(
    post,
    path = "/public/{id}/verify",
    tags = [Share::ENTITY_NAME_PLURAL, "internal"],
    params(("id" = Uuid, Path, description = "Share ID")),
    request_body = String,
    responses(
        (status = 200, description = "Password verified", body = ApiResponse<bool>),
        (status = 401, description = "Invalid password", body = ApiErrorResponse),
        (status = 404, description = "Share not found", body = ApiErrorResponse),
    )
)]
async fn verify_share_password(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(password): Json<String>,
) -> ApiResult<Json<ApiResponse<bool>>> {
    check_share_rate_limit(&id)?;

    let share = state
        .services
        .share_service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Share>(id))?;

    if !share.is_valid() {
        return Err(ApiError::entity_disabled::<Share>());
    }

    if !share.requires_password() {
        return Err(ApiError::bad_request("Share does not require a password"));
    }

    // Verify password - returns error if invalid
    state
        .services
        .share_service
        .verify_share_password(&share, &password)
        .map_err(|_| ApiError::share_password_incorrect())?;

    Ok(Json(ApiResponse::success(true)))
}

/// Get topology data for a public share
async fn get_share_topology(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(query): Query<ShareQuery>,
    req_headers: HeaderMap,
    Json(body): Json<ShareTopologyRequest>,
) -> ApiResult<Response> {
    let share = state
        .services
        .share_service
        .get_by_id(&id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Share>(id))?;

    if !share.is_valid() {
        return Err(ApiError::entity_disabled::<Share>());
    }

    // Get org's plan to check embed feature
    let plan = get_share_org_plan(&state, &share).await?;
    let has_embeds_feature = plan.features().embeds;

    // If requesting embed mode, check if org has embeds feature
    if query.embed && !has_embeds_feature {
        return Err(ApiError::payment_required(
            "Embed access requires a plan with embeds feature",
        ));
    }

    // Handle password-protected shares
    if share.requires_password() {
        check_share_rate_limit(&id)?;
        match &body.password {
            Some(password) => {
                state
                    .services
                    .share_service
                    .verify_share_password(&share, password)
                    .map_err(|_| ApiError::share_password_incorrect())?;
            }
            None => {
                return Err(ApiError::share_password_required());
            }
        }
    }

    // Validate allowed_domains only for embed requests
    if query.embed && share.has_domain_restrictions() {
        let referer = req_headers
            .get(header::REFERER)
            .and_then(|v| v.to_str().ok());

        if !state
            .services
            .share_service
            .validate_allowed_domains(&share, referer)
        {
            let domain = referer.unwrap_or("unknown").to_string();
            return Err(ApiError::coded(
                StatusCode::FORBIDDEN,
                ErrorCode::ShareDomainNotAllowed { domain },
            ));
        }
    }

    // Get topology data
    let topology = state
        .services
        .topology_service
        .storage()
        .get_by_id(&share.base.topology_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Topology>(share.base.topology_id))?;

    let response_data = ShareWithTopology {
        share: PublicShareMetadata::from(&share),
        topology: serde_json::to_value(&topology)
            .map_err(|e| ApiError::internal_error(&e.to_string()))?,
    };

    // Build response with appropriate headers
    let mut response = Json(ApiResponse::success(response_data)).into_response();
    let headers = response.headers_mut();

    // Add cache header
    headers.insert(
        header::CACHE_CONTROL,
        "public, max-age=300".parse().unwrap(),
    );

    // Set CSP frame-ancestors to control iframe embedding
    // This overrides the global 'frame-ancestors self' default
    let frame_ancestors = if has_embeds_feature {
        // Org has embed feature - allow based on allowed_domains
        if let Some(ref domains) = share.base.allowed_domains {
            // Defense-in-depth: filter out any domains with unsafe CSP characters
            let safe_domains: Vec<&String> = domains
                .iter()
                .filter(|d| validate_csp_domain(d).is_ok())
                .collect();
            if !safe_domains.is_empty() {
                // Specific domains allowed
                format!(
                    "frame-ancestors {}",
                    safe_domains
                        .iter()
                        .map(|d| d.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            } else {
                // Empty list = allow all
                "frame-ancestors *".to_string()
            }
        } else {
            // No restrictions = allow all
            "frame-ancestors *".to_string()
        }
    } else {
        // No embed feature - block all framing
        "frame-ancestors 'none'".to_string()
    };

    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        frame_ancestors.parse().unwrap(),
    );

    Ok(response)
}
