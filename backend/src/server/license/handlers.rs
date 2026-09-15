//! License key endpoints. Org owners copy and regenerate the keys for their
//! self-hosted servers; those servers exchange an online key for an
//! entitlement (the contract in [`super::online`]).

use std::num::NonZeroU32;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use chrono::Utc;
use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DashMapStateStore};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use super::mint::{LicenseIssuer, MintError};
use super::online::{EntitlementRequest, EntitlementResponse};
use super::types::LicenseKeyType;
use crate::server::auth::middleware::permissions::{Authorized, Owner};
use crate::server::config::AppState;
use crate::server::openapi::tags as api_tags;
use crate::server::organizations::r#impl::base::Organization;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse,
};

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_entitlement))
        .routes(routes!(create_license_key))
        .routes(routes!(regenerate_license_key))
}

/// The license key to mint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateLicenseKeyRequest {
    pub key_type: LicenseKeyType,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LicenseKeyResponse {
    /// Signed key to set as `SCANOPY_LICENSE_KEY` on a self-hosted server.
    pub key: String,
}

type OrgRateLimiter = RateLimiter<Uuid, DashMapStateStore<Uuid>, DefaultClock>;

/// Per-org cap on entitlement requests. Keyed only after the key's signature
/// checks out, so unsigned junk never reaches a real org's quota (the global
/// per-IP limiter covers that). One request per 10 seconds sustained, bursts
/// of 10: room for several servers sharing one org's key.
fn entitlement_limiter() -> &'static OrgRateLimiter {
    static LIMITER: OnceLock<OrgRateLimiter> = OnceLock::new();
    LIMITER.get_or_init(|| {
        RateLimiter::dashmap(
            Quota::with_period(Duration::from_secs(10))
                .unwrap()
                .allow_burst(NonZeroU32::new(10).unwrap()),
        )
    })
}

fn license_issuer(state: &AppState) -> Result<&LicenseIssuer, ApiError> {
    state
        .license_issuer
        .as_deref()
        .ok_or_else(|| ApiError::internal_error("License signing is not configured on this server"))
}

fn mint_error(error: MintError) -> ApiError {
    match error {
        MintError::NotLicensed | MintError::OfflineNotIncluded | MintError::NoPaidThrough => {
            ApiError::forbidden(&error.to_string())
        }
        MintError::KeyVersionOutOfRange | MintError::Signing(_) => {
            ApiError::internal_error(&error.to_string())
        }
    }
}

/// Exchange an online license key for an entitlement
///
/// Called by self-hosted Scanopy servers. The key is the credential, so the
/// endpoint takes no session or API key. Every success records the check-in
/// and returns an entitlement minted from the organization's current plan and
/// paid-through date.
#[utoipa::path(
    post,
    path = "/entitlement",
    tags = [api_tags::BILLING, api_tags::INTERNAL],
    request_body = EntitlementRequest,
    responses(
        (status = 200, description = "Entitlement for the key's organization", body = ApiResponse<EntitlementResponse>),
        (status = 400, description = "Malformed key, bad signature, or not an online key", body = ApiErrorResponse),
        (status = 403, description = "Key was regenerated, or the organization is not on a self-hosted plan", body = ApiErrorResponse),
        (status = 429, description = "Too many requests for this organization", body = ApiErrorResponse),
    )
)]
pub async fn get_entitlement(
    State(state): State<Arc<AppState>>,
    Json(request): Json<EntitlementRequest>,
) -> ApiResult<Json<ApiResponse<EntitlementResponse>>> {
    let issuer = license_issuer(&state)?;
    let claims = issuer
        .decode_online_key(&request.key)
        .map_err(|e| ApiError::bad_request(&e.to_string()))?;
    let organization_id = Uuid::parse_str(&claims.org_id)
        .map_err(|_| ApiError::bad_request("License key names an invalid organization"))?;

    if entitlement_limiter().check_key(&organization_id).is_err() {
        return Err(ApiError::too_many_requests(
            "Too many entitlement requests for this organization. Try again shortly.".to_string(),
        ));
    }

    let service = &state.services.organization_service;
    let organization = service
        .get_by_id(&organization_id)
        .await?
        .ok_or_else(|| ApiError::forbidden("This license key's organization no longer exists"))?;

    if i64::from(claims.key_version) != organization.base.license_key_version {
        return Err(ApiError::forbidden(
            "This license key was regenerated. Copy the current key from Scanopy Cloud.",
        ));
    }

    let now = Utc::now();
    let entitlement = issuer
        .mint_entitlement(&organization, now)
        .map_err(mint_error)?;
    service
        .record_license_check_in(organization_id, now)
        .await?;

    Ok(Json(ApiResponse::success(EntitlementResponse {
        entitlement,
    })))
}

/// Mint a license key for this organization's self-hosted servers
///
/// Online keys work on any plan with a self-hosted license. Offline keys need
/// a plan with air-gapped deployment.
#[utoipa::path(
    post,
    path = "/keys",
    tags = [api_tags::BILLING, api_tags::INTERNAL],
    request_body = CreateLicenseKeyRequest,
    responses(
        (status = 200, description = "Signed license key", body = ApiResponse<LicenseKeyResponse>),
        (status = 403, description = "Not an owner, or the plan does not include this key type", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn create_license_key(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(request): Json<CreateLicenseKeyRequest>,
) -> ApiResult<Json<ApiResponse<LicenseKeyResponse>>> {
    let organization_id = auth.require_organization_id()?;
    let issuer = license_issuer(&state)?;
    let organization = state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(organization_id))?;

    let key = issuer
        .mint_key(&organization, request.key_type, Utc::now())
        .map_err(mint_error)?;

    Ok(Json(ApiResponse::success(LicenseKeyResponse { key })))
}

/// Regenerate this organization's online license key
///
/// Retires every online key issued so far: servers still using one get 403
/// from the entitlement endpoint.
#[utoipa::path(
    post,
    path = "/keys/regenerate",
    tags = [api_tags::BILLING, api_tags::INTERNAL],
    responses(
        (status = 200, description = "Key regenerated", body = EmptyApiResponse),
        (status = 403, description = "Not an owner", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn regenerate_license_key(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
) -> ApiResult<Json<EmptyApiResponse>> {
    let organization_id = auth.require_organization_id()?;
    state
        .services
        .organization_service
        .regenerate_license_key(organization_id, auth.entity.clone())
        .await?;

    Ok(Json(ApiResponse::success(())))
}
