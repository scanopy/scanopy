use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::auth::middleware::permissions::{Authorized, IsUser, Member, Owner};
use crate::server::config::AppState;
use crate::server::openapi::tags as api_tags;
use crate::server::organizations::demo_seed::seed_demo_data;
use crate::server::organizations::demo_status::DemoPopulateStatus;
use crate::server::organizations::r#impl::base::Organization;
use crate::server::shared::events::traits::{Event, OrgScope};
use crate::server::shared::events::types::{OnboardingOperation, OnboardingOperationDiscriminants};
use crate::server::shared::handlers::traits::{CrudHandlers, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::{Entity, Storage};
use crate::server::shared::types::api::ApiJson;
use crate::server::shared::types::api::ApiResponse;
use crate::server::shared::types::api::ApiResult;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, EmptyApiResponse};
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::sites::r#impl::Site;
use crate::server::users::r#impl::base::User;
use crate::server::users::r#impl::permissions::UserOrgPermissions;
use anyhow::anyhow;
use axum::Json;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;
use std::sync::Arc;
use strum::IntoEnumIterator;
use tower_sessions::Session;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

pub const DEMO_USER_ID: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440050);

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_organization, update_org_name))
        .routes(routes!(update_profile))
        .routes(routes!(submit_referral_source))
        .routes(routes!(daemon_prompt_response))
        .routes(routes!(reset))
        .routes(routes!(delete_organization))
        .routes(routes!(populate_demo_data))
        .routes(routes!(populate_demo_status))
}

/// Get the current user's organization
#[utoipa::path(
    get,
    path = "",
    tag = Organization::ENTITY_NAME_PLURAL,
    responses(
        (status = 200, description = "Organization details", body = ApiResponse<Organization>),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
    security(("session" = []))
)]
pub async fn get_organization(
    State(state): State<Arc<AppState>>,
    auth: Authorized<IsUser>,
) -> ApiResult<Json<ApiResponse<Organization>>> {
    let organization_id = auth.require_organization_id()?;
    let service = Organization::get_service(&state);
    let entity = service
        .get_by_id(&organization_id)
        .await
        .map_err(|e| ApiError::internal_error(&e.to_string()))?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(organization_id))?;

    Ok(Json(ApiResponse::success(entity)))
}

/// Update organization name
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Organization::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Organization ID")),
    request_body = String,
    responses(
        (status = 200, description = "Organization updated", body = ApiResponse<Organization>),
        (status = 403, description = "Only owners can update organization", body = ApiErrorResponse),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn update_org_name(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
    ApiJson(name): ApiJson<String>,
) -> ApiResult<Json<ApiResponse<Organization>>> {
    // Organization's scoping fields are both None, so the generic update_handler
    // below cannot enforce tenant ownership — check it explicitly here, matching
    // the sibling reset/delete/populate-demo handlers.
    let user_org_id = auth.require_organization_id()?;
    if id != user_org_id {
        return Err(ApiError::permission_denied());
    }

    let mut org = state
        .services
        .organization_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| anyhow!("Could not find org"))?;

    org.base.name = name;

    update_handler::<Organization>(
        axum::extract::State(state),
        auth.into_permission::<Member>(),
        axum::extract::Path(id),
        ApiJson(org),
    )
    .await
}

/// Request to update user profile (deferred marketing fields)
#[derive(Debug, Deserialize, ToSchema)]
pub struct ProfileUpdateRequest {
    /// The user's job title, collected during onboarding.
    pub job_title: Option<String>,
    /// Company size bracket, collected during onboarding.
    pub company_size: Option<String>,
}

/// Update user profile with deferred marketing fields
#[utoipa::path(
    post,
    path = "/profile",
    tag = Organization::ENTITY_NAME_PLURAL,
    request_body = ProfileUpdateRequest,
    responses(
        (status = 200, description = "Profile updated", body = EmptyApiResponse),
    )
)]
async fn update_profile(
    auth: Authorized<IsUser>,
    State(state): State<Arc<AppState>>,
    ApiJson(request): ApiJson<ProfileUpdateRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let org_id = auth.organization_id().unwrap();
    let authentication: AuthenticatedEntity = auth.into();

    state
        .services
        .event_bus
        .publish(Event::new(
            OrgScope {
                organization_id: org_id,
            },
            OnboardingOperation::ProfileCompleted {
                job_title: request.job_title,
                company_size: request.company_size,
            },
            authentication,
        ))
        .await
        .map_err(|e| {
            ApiError::internal_error(&format!("Failed to publish profile event: {}", e))
        })?;

    Ok(Json(ApiResponse::success(())))
}

/// How a user first heard about Scanopy, as offered by the onboarding prompt.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReferralSource {
    SearchEngine,
    AiAssistant,
    Youtube,
    Tiktok,
    BlogArticle,
    Reddit,
    HackerNews,
    SocialMedia,
    WordOfMouth,
    ProxmoxCommunityScripts,
    SelfHosted,
    Other,
    PreferNotToSay,
}

/// Request to submit referral source
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReferralSourceRequest {
    /// How the user heard about Scanopy.
    pub referral_source: ReferralSource,
    /// Free-text detail, sent when `referral_source` is `other`.
    pub referral_source_other: Option<String>,
}

/// Submit referral source (how did you hear about us)
#[utoipa::path(
    post,
    path = "/referral-source",
    tag = Organization::ENTITY_NAME_PLURAL,
    request_body = ReferralSourceRequest,
    responses(
        (status = 200, description = "Referral source recorded", body = EmptyApiResponse),
    )
)]
async fn submit_referral_source(
    auth: Authorized<IsUser>,
    State(state): State<Arc<AppState>>,
    ApiJson(request): ApiJson<ReferralSourceRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let org_id = auth.organization_id().unwrap();
    let authentication: AuthenticatedEntity = auth.into();

    state
        .services
        .event_bus
        .publish(Event::new(
            OrgScope {
                organization_id: org_id,
            },
            OnboardingOperation::ReferralSourceCompleted {
                referral_source: request.referral_source,
                referral_source_other: request.referral_source_other,
            },
            authentication,
        ))
        .await
        .map_err(|e| {
            ApiError::internal_error(&format!("Failed to publish referral source event: {}", e))
        })?;

    Ok(Json(ApiResponse::success(())))
}

/// Which daemon-prompt CTA the user chose.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DaemonPromptAction {
    Dismissed,
    Accepted,
}

/// Request recording the user's response to the "Start Discovering Your Network" prompt.
#[derive(Debug, Deserialize, ToSchema)]
pub struct DaemonPromptResponseRequest {
    /// What the user chose to do about the daemon prompt.
    pub action: DaemonPromptAction,
}

/// Record the user's response to the daemon-install prompt so it is not shown again.
/// Each CTA persists a distinct onboarding milestone (the org subscriber dedups); the
/// PostHog subscriber turns these into funnel events, so no client-side telemetry is needed.
#[utoipa::path(
    post,
    path = "/daemon-prompt-response",
    tag = Organization::ENTITY_NAME_PLURAL,
    request_body = DaemonPromptResponseRequest,
    responses(
        (status = 200, description = "Response recorded", body = EmptyApiResponse),
    )
)]
async fn daemon_prompt_response(
    auth: Authorized<Member>,
    State(state): State<Arc<AppState>>,
    ApiJson(request): ApiJson<DaemonPromptResponseRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let org_id = auth.organization_id().unwrap();
    let authentication: AuthenticatedEntity = auth.into();

    let operation = match request.action {
        DaemonPromptAction::Dismissed => OnboardingOperation::DaemonPromptDismissed,
        DaemonPromptAction::Accepted => OnboardingOperation::DaemonPromptAccepted,
    };

    state
        .services
        .event_bus
        .publish(Event::new(
            OrgScope {
                organization_id: org_id,
            },
            operation,
            authentication,
        ))
        .await
        .map_err(|e| {
            ApiError::internal_error(&format!("Failed to publish daemon prompt event: {}", e))
        })?;

    Ok(Json(ApiResponse::success(())))
}

/// Reset all organization data (delete all entities except organization and owner user)
#[utoipa::path(
    post,
    path = "/{id}/reset",
    tags = [Organization::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Organization reset", body = EmptyApiResponse),
        (status = 403, description = "Cannot reset another organization", body = ApiErrorResponse),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn reset(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let user_org_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    // Verify organization exists
    let org = state
        .services
        .organization_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(id))?;

    if org.id != user_org_id {
        return Err(ApiError::permission_denied());
    }

    let entity: AuthenticatedEntity = auth.into_entity();

    reset_organization_data(&state, &org.id, entity.clone()).await?;

    // Recreate the default site, its subnets and topology so the org always has one
    state
        .services
        .topology_service
        .ensure_cloud_setup(org.id, None, entity)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to set up site: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

/// Delete the organization entirely, including all data and users
#[utoipa::path(
    delete,
    path = "/{id}",
    tags = [Organization::ENTITY_NAME_PLURAL],
    params(("id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Organization deleted", body = EmptyApiResponse),
        (status = 403, description = "Cannot delete another organization", body = ApiErrorResponse),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
    ),
     security(("session" = []))
)]
pub async fn delete_organization(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
    session: Session,
) -> ApiResult<Json<ApiResponse<()>>> {
    let user_org_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    // Verify organization exists
    let org = state
        .services
        .organization_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(id))?;

    if org.id != user_org_id {
        return Err(ApiError::permission_denied());
    }

    let has_active_paid_sub = if let Some(billing) = &state.services.billing_service {
        billing.has_active_paid_subscription(org.id).await?
    } else {
        false
    };
    if has_active_paid_sub {
        return Err(ApiError::coded(
            axum::http::StatusCode::CONFLICT,
            ErrorCode::OrganizationHasActiveSubscription,
        ));
    }

    let entity: AuthenticatedEntity = auth.into_entity();

    // Best-effort Stripe teardown. Stripe auto-cancels active subscriptions
    // on customer-delete; failures here don't block deletion — the webhook
    // handlers no-op on missing org if Stripe still fires events afterwards.
    if let (Some(customer_id), Some(billing)) = (
        org.base.stripe_customer_id.as_deref(),
        state.services.billing_service.as_ref(),
    ) && let Err(e) = billing.delete_stripe_customer(customer_id).await
    {
        tracing::error!(
            organization_id = %org.id,
            stripe_customer_id = %customer_id,
            error = %e,
            "Failed to delete Stripe customer during org deletion — proceeding"
        );
    }

    // 1. Delete all child entities (reuse reset logic)
    reset_organization_data(&state, &org.id, entity.clone()).await?;

    // 2. Delete ALL users (including owner)
    let user_filter = StorableFilter::<User>::new_from_org_id(&org.id);
    let all_user_ids: Vec<Uuid> = state
        .services
        .user_service
        .get_all(user_filter)
        .await?
        .iter()
        .map(|u| u.id)
        .collect();

    // Org-deleted confirmation email is dispatched by the email subscriber
    // reacting to `EntityOperation::Deleted` for `Entity::Organization`. The
    // event's `authentication` field carries the initiating user's email.

    if !all_user_ids.is_empty() {
        state
            .services
            .user_service
            .storage()
            .delete_many(&all_user_ids)
            .await?;
    }

    // 3. Delete the organization itself via the CRUD service so the
    //    `EntityOperation::Deleted` event fires; the email subscriber for
    //    `Entity::Organization { Deleted }` dispatches the confirmation
    //    email to the initiating user (carried on `event.authentication`).
    state
        .services
        .organization_service
        .delete(&org.id, entity)
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to delete organization: {}", e)))?;

    // 4. Invalidate caller's session
    session
        .delete()
        .await
        .map_err(|e| ApiError::internal_error(&format!("Failed to delete session: {}", e)))?;

    Ok(Json(ApiResponse::success(())))
}

/// Populate demo data (only available for demo organizations).
///
/// Runs the population off the request thread (a `tokio::spawn`) and returns
/// `202` immediately — the work is a few hundred sequential DB round-trips and
/// would otherwise exceed the reverse-proxy request timeout against a remote
/// database. Poll `GET /{id}/populate-demo/status` for completion/failure.
#[utoipa::path(
    post,
    path = "/{id}/populate-demo",
    tags = [Organization::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 202, description = "Demo data population started", body = ApiResponse<DemoPopulateStatus>),
        (status = 403, description = "Only available for demo organizations", body = ApiErrorResponse),
        (status = 404, description = "Organization not found", body = ApiErrorResponse),
        (status = 409, description = "Population already in progress", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
pub async fn populate_demo_data(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
) -> ApiResult<(StatusCode, Json<ApiResponse<DemoPopulateStatus>>)> {
    let user_org_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id().ok_or_else(ApiError::user_required)?;

    let org = state
        .services
        .organization_service
        .get_by_id(&id)
        .await?
        .ok_or_else(|| ApiError::entity_not_found::<Organization>(id))?;

    if org.id != user_org_id {
        return Err(ApiError::permission_denied());
    }

    // Only available for demo organizations
    let plan = org
        .base
        .plan
        .unwrap_or_else(crate::server::billing::plans::get_free_plan);
    if !plan.is_demo() {
        return Err(ApiError::forbidden(
            "Populate demo data is only available for demo organizations",
        ));
    }

    let entity: AuthenticatedEntity = auth.into_entity();

    // Single-flight per org: a retry while a run is in flight must not stack a
    // second population (it would race the reset). `try_begin_demo` sets the
    // `Running` status we hand back in the 202 body.
    let Some(started) = state.services.organization_service.try_begin_demo(id).await else {
        return Err(ApiError::conflict(
            "Demo data population is already in progress for this organization",
        ));
    };

    let task_state = state.clone();
    tokio::spawn(async move {
        let org_service = task_state.services.organization_service.clone();
        let terminal = match run_populate_demo(task_state.clone(), id, user_id, entity, org).await {
            Ok(()) => DemoPopulateStatus::Complete {
                finished_at: chrono::Utc::now(),
            },
            Err(e) => {
                tracing::error!(
                    organization_id = %id,
                    error = %e,
                    "Demo data population failed",
                );
                DemoPopulateStatus::Failed {
                    error: e.to_string(),
                    finished_at: chrono::Utc::now(),
                }
            }
        };
        org_service.set_demo_status(id, terminal).await;
    });

    Ok((StatusCode::ACCEPTED, Json(ApiResponse::success(started))))
}

/// Poll the status of an org's background demo-populate task.
#[utoipa::path(
    get,
    path = "/{id}/populate-demo/status",
    tags = [Organization::ENTITY_NAME_PLURAL, api_tags::INTERNAL],
    params(("id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "Demo populate status", body = ApiResponse<DemoPopulateStatus>),
        (status = 404, description = "No demo-populate task for this organization", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
pub async fn populate_demo_status(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ApiResponse<DemoPopulateStatus>>> {
    let user_org_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    if id != user_org_id {
        return Err(ApiError::permission_denied());
    }
    let status = state
        .services
        .organization_service
        .get_demo_status(&id)
        .await
        .ok_or_else(|| {
            ApiError::not_found("No demo-populate task for this organization".to_string())
        })?;
    Ok(Json(ApiResponse::success(status)))
}

/// The population work itself — runs in the spawned task, off the request
/// thread. Returns `Ok(())` on success; the caller records the terminal status.
async fn run_populate_demo(
    state: Arc<AppState>,
    id: Uuid,
    user_id: Uuid,
    entity: AuthenticatedEntity,
    mut org: Organization,
) -> ApiResult<()> {
    // First, reset all existing data
    reset_organization_data(&state, &id, entity.clone()).await?;

    org.base.onboarding = OnboardingOperationDiscriminants::iter().collect();

    state
        .services
        .organization_service
        .update(&mut org, entity.clone())
        .await?;

    seed_demo_data(&state.services, id, user_id, entity).await
}

/// Internal function to reset organization data (reused by populate_demo_data).
///
/// Uses direct storage-level bulk deletes instead of service-level `delete_all_for_org`
/// to avoid O(N) per-entity tag removal and event publishing. This is safe because:
/// - We're deleting the entire org's data, not selective entities
/// - Tags are deleted first, and `tag_id REFERENCES tags(id) ON DELETE CASCADE`
///   automatically cleans up all entity_tags — no per-entity removal needed
/// - Event publishing during a full demo reset is unnecessary
async fn reset_organization_data(
    state: &Arc<AppState>,
    organization_id: &Uuid,
    _auth: AuthenticatedEntity,
) -> Result<(), ApiError> {
    use crate::server::credentials::r#impl::base::Credential;
    use crate::server::invites::r#impl::base::Invite;
    use crate::server::tags::r#impl::base::Tag;
    use crate::server::user_api_keys::r#impl::base::UserApiKey;

    // Deletes run sequentially: several of these cascades overlap on shared
    // junction rows (e.g. networks and user_api_keys both cascade
    // user_api_key_site_access; tags cascade entity_tags), so running them
    // concurrently risks lock-ordering deadlocks for negligible gain — the
    // reset is a handful of deletes and was never the slow part.

    // 1. Delete tags — CASCADE on tag_id cleans up entity_tags automatically.
    state
        .services
        .tag_service
        .storage()
        .delete_by_filter(StorableFilter::<Tag>::new_from_org_id(organization_id))
        .await?;

    // 2. Delete sites — CASCADE handles all site-scoped entities
    //    (hosts, services, subnets, topologies, shares, daemons, discoveries,
    //    ports, bindings, interfaces, IP addresses, daemon API keys, etc.)
    state
        .services
        .site_service
        .storage()
        .delete_by_filter(StorableFilter::<Site>::new_from_org_id(organization_id))
        .await?;

    // 3. Delete org-scoped entities not tied to sites
    state
        .services
        .user_api_key_service
        .storage()
        .delete_by_filter(StorableFilter::<UserApiKey>::new_from_org_id(
            organization_id,
        ))
        .await?;
    state
        .services
        .invite_service
        .storage()
        .delete_by_filter(StorableFilter::<Invite>::new_from_org_id(organization_id))
        .await?;
    state
        .services
        .credential_service
        .storage()
        .delete_by_filter(StorableFilter::<Credential>::new_from_org_id(
            organization_id,
        ))
        .await?;

    // 4. Delete non-owner users
    let user_filter = StorableFilter::<User>::new_from_org_id(organization_id);
    let non_owner_user_ids: Vec<Uuid> = state
        .services
        .user_service
        .get_all(user_filter)
        .await?
        .iter()
        .filter_map(|u| {
            if u.base.permissions != UserOrgPermissions::Owner {
                Some(u.id)
            } else {
                None
            }
        })
        .collect();

    if !non_owner_user_ids.is_empty() {
        state
            .services
            .user_service
            .storage()
            .delete_many(&non_owner_user_ids)
            .await?;
    }

    Ok(())
}
