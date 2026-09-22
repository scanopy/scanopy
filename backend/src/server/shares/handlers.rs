use crate::server::openapi::tags as api_tags;
use std::{num::NonZeroU32, sync::Arc};

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderValue, header},
    response::{Html, IntoResponse, Response},
};
use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DashMapStateStore};
use secrecy::ExposeSecret;
use serde::Deserialize;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::{
    auth::{
        middleware::{
            auth::AuthenticatedEntity,
            features::{RequireFeature, ShareViewsFeature},
            permissions::{Authorized, Member, RequireVerified},
        },
        service::hash_password,
    },
    billing::types::base::BillingPlan,
    config::AppState,
    credentials::r#impl::types::REDACTED_SECRET_SENTINEL,
    networks::r#impl::Network,
    shared::validation::validate_csp_domain,
    shared::{
        events::{
            traits::{Event, OrgScope},
            types::AnalyticsOperation,
        },
        handlers::traits::{CrudHandlers, create_handler, update_handler},
        services::traits::CrudService,
        storage::traits::Entity,
        types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult},
    },
    shares::r#impl::{
        api::{
            CreateUpdateShareRequest, ExportFeatures, PublicShareMetadata,
            ShareAccessTokenResponse, ShareWithTopology,
        },
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

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ShareQuery {
    /// Return the share prepared for embedding in another page.
    #[serde(default)]
    pub embed: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ShareTopologyRequest {
    /// Server-issued access token obtained from `/verify`. Required when the share has a password.
    #[serde(default)]
    pub access_token: Option<String>,
    /// Which topology view to return data for
    pub view: crate::server::topology::types::views::TopologyView,
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
        .routes(routes!(get_share_topology))
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
    auth: Authorized<RequireVerified<Member>>,
    Json(CreateUpdateShareRequest { mut share }): Json<CreateUpdateShareRequest>,
) -> ApiResult<Json<ApiResponse<Share>>> {
    // Validate allowed_domains for CSP safety
    if let Some(ref domains) = share.base.allowed_domains {
        for domain in domains {
            validate_csp_domain(domain)?;
        }
    }

    // Password handling — the field round-trips as the redaction sentinel, so on
    // create we only hash when the client sent a real plaintext value. Sentinel
    // and empty both mean "no password".
    share.base.password_hash = match share
        .base
        .password
        .as_ref()
        .map(ExposeSecret::expose_secret)
    {
        Some(plaintext) if !plaintext.is_empty() && plaintext != REDACTED_SECRET_SENTINEL => {
            Some(hash_password(plaintext).map_err(|e| ApiError::internal_error(&e.to_string()))?)
        }
        _ => None,
    };
    share.base.redact_password();

    share.base.created_by = auth.user_id().ok_or_else(ApiError::user_required)?;

    create_handler::<Share>(State(state), auth.into_permission::<Member>(), Json(share)).await
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
    auth: Authorized<RequireVerified<Member>>,
    Path(id): Path<Uuid>,
    Json(CreateUpdateShareRequest { mut share }): Json<CreateUpdateShareRequest>,
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

    // Password handling — `share.base.password` round-trips as the redaction
    // sentinel, so:
    // - None or Some(sentinel): preserve the existing hash (user didn't touch it).
    // - Some(""): client explicitly cleared the field → remove the password.
    // - Some(plaintext): hash and set the new password.
    share.base.password_hash = match share
        .base
        .password
        .as_ref()
        .map(ExposeSecret::expose_secret)
    {
        None => existing.base.password_hash,
        Some(v) if v == REDACTED_SECRET_SENTINEL => existing.base.password_hash,
        Some("") => None,
        Some(plaintext) => {
            Some(hash_password(plaintext).map_err(|e| ApiError::internal_error(&e.to_string()))?)
        }
    };
    share.base.redact_password();

    // Delegate to generic handler
    update_handler::<Share>(
        State(state),
        auth.into_permission::<Member>(),
        Path(id),
        Json(share),
    )
    .await
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
        .get_by_id(&share.base.network_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Network>(share.base.network_id))?;

    Ok(state
        .services
        .organization_service
        .get_by_id(&network.base.organization_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .and_then(|o| o.base.plan)
        .unwrap_or_else(crate::server::billing::plans::get_free_plan))
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
        (status = 403, description = "Share is disabled or expired", body = ApiErrorResponse),
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

    // Fetch topology to resolve available views based on data
    let topology = state
        .services
        .topology_service
        .get_by_id(&share.base.topology_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Topology>(share.base.topology_id))?;

    // Compute view-support from raw entity tables so availability doesn't
    // flap based on which view the main app last rebuilt under.
    let support = state
        .services
        .topology_service
        .get_view_support(topology.base.network_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    let enabled_views = topology.resolve_available_views(&share.base.enabled_views, &support);

    Ok(Json(ApiResponse::success(PublicShareMetadata::new(
        &share,
        enabled_views,
    ))))
}

/// Verify password for a password-protected share and return an access token.
///
/// The returned token is an HS256 JWT tied to the share's current password
/// hash; subsequent `/topology` calls send the token instead of the raw
/// password. Changing the share password invalidates outstanding tokens.
#[utoipa::path(
    post,
    path = "/public/{id}/verify",
    tags = [Share::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Share ID")),
    request_body = String,
    responses(
        (status = 200, description = "Password verified; access token issued", body = ApiResponse<ShareAccessTokenResponse>),
        (status = 400, description = "Share has no password to verify", body = ApiErrorResponse),
        (status = 401, description = "Invalid password", body = ApiErrorResponse),
        (status = 403, description = "Share is disabled or expired", body = ApiErrorResponse),
        (status = 404, description = "Share not found", body = ApiErrorResponse),
        (status = 429, description = "Too many attempts for this share", body = ApiErrorResponse),
    )
)]
async fn verify_share_password(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(password): Json<String>,
) -> ApiResult<Json<ApiResponse<ShareAccessTokenResponse>>> {
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

    let issued = state.services.share_service.issue_access_token(&share)?;

    Ok(Json(ApiResponse::success(ShareAccessTokenResponse {
        access_token: issued.token,
        expires_at: issued.expires_at,
    })))
}

/// Get topology data for a public share
#[utoipa::path(
    post,
    path = "/public/{id}/topology",
    tags = [Share::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Share ID"), ShareQuery),
    request_body = ShareTopologyRequest,
    responses(
        (status = 200, description = "Share metadata and topology data", body = ApiResponse<ShareWithTopology>),
        (status = 400, description = "View is not enabled on this share", body = ApiErrorResponse),
        (status = 401, description = "Access token missing, expired or tampered with", body = ApiErrorResponse),
        (status = 402, description = "Embedding is not included in the share owner's plan", body = ApiErrorResponse),
        (status = 403, description = "Share is disabled or expired, or its password was not verified", body = ApiErrorResponse),
        (status = 404, description = "Share not found", body = ApiErrorResponse),
        (status = 429, description = "Too many requests for this share", body = ApiErrorResponse),
    )
)]
async fn get_share_topology(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(query): Query<ShareQuery>,
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

    // Get org's plan to check embed feature and export permissions
    let plan = get_share_org_plan(&state, &share).await?;
    let plan_features = plan.features();
    let has_embeds_feature = plan_features.embeds;

    // If requesting embed mode, check if org has embeds feature
    if query.embed && !has_embeds_feature {
        return Err(ApiError::payment_required(
            "Embed access requires a plan with embeds feature",
        ));
    }

    // Handle password-protected shares: require a valid access token (issued
    // by `/verify`) in place of the raw password.
    if share.requires_password() {
        check_share_rate_limit(&id)?;
        match &body.access_token {
            Some(token) => {
                state
                    .services
                    .share_service
                    .verify_access_token(&share, token)?;
            }
            None => {
                return Err(ApiError::share_password_required());
            }
        }
    }

    // Parent-origin restriction for embeds is enforced by
    // `Content-Security-Policy: frame-ancestors` on the HTML response
    // (see `share_html_handler`). The browser blocks disallowed framings
    // before the iframe renders; the Referer header never carries the
    // parent frame's origin for same-origin iframe fetches, so a
    // server-side referer check here cannot enforce what it appears to.

    // Get topology data
    let topology = state
        .services
        .topology_service
        .get_by_id(&share.base.topology_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Topology>(share.base.topology_id))?;

    // Resolve available views based on share config + data availability.
    // Support flags come from raw entity tables — independent of whichever
    // view the topology was last rebuilt under.
    let support = state
        .services
        .topology_service
        .get_view_support(topology.base.network_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    let enabled_views = topology.resolve_available_views(&share.base.enabled_views, &support);

    // Validate requested view is available
    if !enabled_views.contains(&body.view) {
        return Err(ApiError::bad_request(&format!(
            "View {:?} is not available for this share",
            body.view
        )));
    }

    // Load entity data for the topology. Today shares only target live-view
    // topologies; snapshot-pinned shares are deferred until the snapshots
    // service is fully wired into AppState.
    let service = &state.services.topology_service;
    let network_id = topology.base.network_id;
    // Build the per-view graph on request from current entities + the network's
    // grouping options (the graph is no longer persisted).
    let data = service
        .get_topology_render_data(network_id, None)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;

    let export_features = ExportFeatures {
        png_export: plan_features.png_export,
        svg_export: plan_features.svg_export,
        mermaid_export: plan_features.mermaid_export,
        confluence_export: plan_features.confluence_export,
        pdf_export: plan_features.pdf_export,
        html_export: plan_features.html_export,
        remove_created_with: plan_features.remove_created_with,
    };

    // Return the slim topology row + the built bundle; the share viewer composes
    // them with the same `toRenderableTopology` the app uses (no server-side
    // merge). `data` already carries the per-view graph from
    // `get_topology_render_data`.
    let response_data = ShareWithTopology {
        share: PublicShareMetadata::new(&share, enabled_views),
        topology,
        data,
        export_features,
    };

    // Track share/embed view via event bus
    {
        let org_id = state
            .services
            .network_service
            .get_by_id(&share.base.network_id)
            .await
            .ok()
            .flatten()
            .map(|n| n.base.organization_id);

        if let Some(org_id) = org_id {
            let has_password = share.requires_password();
            let operation = if query.embed {
                AnalyticsOperation::TopologyEmbedViewed {
                    share_id: id,
                    has_password,
                }
            } else {
                AnalyticsOperation::TopologyShareViewed {
                    share_id: id,
                    has_password,
                }
            };
            let _ = state
                .services
                .event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: org_id,
                    },
                    operation,
                    AuthenticatedEntity::System,
                ))
                .await;
        }
    }

    // Build response with appropriate headers. The per-share
    // `frame-ancestors` CSP directive is set on the HTML response in
    // `share_html_handler` — it has no effect on this JSON response.
    let mut response = Json(ApiResponse::success(response_data)).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "public, max-age=300".parse().unwrap(),
    );

    Ok(response)
}

/// Bytes of the SPA `index.html`, loaded once at startup and shared across
/// all share-HTML requests. Wrapped in `Arc` so the axum Extension layer
/// can clone cheaply.
#[derive(Clone)]
pub struct ShareIndexHtml(pub Arc<String>);

/// Serve the SPA `index.html` for `/share/{id}` and `/share/{id}/embed`
/// with a tight, per-share CSP.
///
/// The per-share `frame-ancestors` directive is derived from the share's
/// `allowed_domains` and the organization's embed feature flag. Unknown
/// shares (or any error determining the plan) fall back to
/// `frame-ancestors 'none'` — safest default.
pub async fn share_html_handler(
    State(state): State<Arc<AppState>>,
    Extension(index): Extension<ShareIndexHtml>,
    Path(id): Path<Uuid>,
) -> Response {
    use crate::server::shares::service::build_frame_ancestors;

    let frame_ancestors = match state.services.share_service.get_by_id(&id).await {
        Ok(Some(share)) => {
            let has_embeds_feature = get_share_org_plan(&state, &share)
                .await
                .map(|p| p.features().embeds)
                .unwrap_or(false);
            build_frame_ancestors(&share, has_embeds_feature)
        }
        _ => "frame-ancestors 'none'".to_string(),
    };

    // Tight CSP for share HTML routes. `connect-src 'self'` is the core
    // defense — even if an XSS vector is found, stolen access tokens or
    // topology data cannot be POSTed to a foreign origin. PostHog is
    // deliberately omitted; share pages do not phone home.
    let csp = format!(
        "default-src 'self'; \
         script-src 'self' 'unsafe-inline'; \
         style-src 'self' 'unsafe-inline'; \
         img-src 'self' data: blob:; \
         font-src 'self'; \
         connect-src 'self'; \
         object-src 'none'; \
         base-uri 'none'; \
         form-action 'none'; \
         {frame_ancestors}"
    );

    let csp_header = HeaderValue::try_from(csp)
        .unwrap_or_else(|_| HeaderValue::from_static("default-src 'none'"));

    let mut response = Html((*index.0).clone()).into_response();
    response
        .headers_mut()
        .insert(header::CONTENT_SECURITY_POLICY, csp_header);
    response
}
