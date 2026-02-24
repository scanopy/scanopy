use crate::server::auth::middleware::permissions::{Authorized, Owner, Viewer};
use crate::server::billing::types::api::{
    ChangePlanPreview, ChangePlanRequest, CreateCheckoutRequest, SetupPaymentMethodRequest,
};
use crate::server::billing::types::base::BillingPlan;
use crate::server::config::AppState;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::types::ErrorCode;
use crate::server::shared::types::api::{ApiError, ApiResult};
use crate::server::shared::types::api::{ApiErrorResponse, ApiResponse, EmptyApiResponse};
use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::header::CACHE_CONTROL;
use axum::response::IntoResponse;
use axum_client_ip::ClientIp;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

/// Enterprise plan inquiry request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnterpriseInquiryRequest {
    /// Contact email
    pub email: String,
    /// Contact name
    pub name: String,
    /// Company name
    pub company: String,
    /// Team/company size: 1-10, 11-25, 26-50, 51-100, 101-250, 251-500, 501-1000, 1001+
    pub team_size: String,
    /// Message/use case description
    pub message: String,
    /// Urgency: immediately, 1-3 months, 3-6 months, exploring
    #[serde(default)]
    pub urgency: Option<String>,
    /// Number of networks/sites
    #[serde(default)]
    pub network_count: Option<i64>,
    /// Plan type being inquired about
    #[serde(default)]
    pub plan_type: Option<String>,
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_billing_plans))
        .routes(routes!(create_checkout_session))
        .routes(routes!(setup_payment_method))
        .routes(routes!(change_plan))
        .routes(routes!(preview_plan_change))
        .routes(routes!(handle_webhook))
        .routes(routes!(create_portal_session))
        .routes(routes!(submit_enterprise_inquiry))
}

/// Get available billing plans
#[utoipa::path(
    get,
    path = "/plans",
    tags = ["billing", "internal"],
    responses(
        (status = 200, description = "List of available billing plans", body = ApiResponse<Vec<BillingPlan>>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_billing_plans(
    State(state): State<Arc<AppState>>,
    _auth: Authorized<Owner>,
) -> Result<impl IntoResponse, ApiError> {
    if let Some(billing_service) = state.services.billing_service.clone() {
        let plans = billing_service.get_plans();
        Ok((
            [(CACHE_CONTROL, "no-store, no-cache, must-revalidate")],
            Json(ApiResponse::success(plans)),
        ))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Create a checkout session
#[utoipa::path(
    post,
    path = "/checkout",
    tags = ["billing", "internal"],
    request_body = CreateCheckoutRequest,
    responses(
        (status = 200, description = "Checkout session URL", body = ApiResponse<String>),
        (status = 400, description = "Invalid plan or billing not enabled", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_checkout_session(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(request): Json<CreateCheckoutRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let current_plans = billing_service.get_plans();

        if !current_plans.contains(&request.plan) {
            return Err(ApiError::validation(ErrorCode::ValidationInvalidFormat {
                field: "plan".to_string(),
            }));
        }

        let success_url = format!("{}?billing_flow=checkout", request.url);

        // Check if org already has a plan — route based on target plan and payment state
        let org = billing_service.get_organization(organization_id).await?;

        if org.base.plan.is_some() && org.base.stripe_customer_id.is_some() {
            if request.plan.is_free() {
                // Downgrade to Free — schedule cancellation at end of billing cycle
                let result = billing_service
                    .schedule_downgrade(organization_id, auth.into_entity())
                    .await?;
                Ok(Json(ApiResponse::success(result)))
            } else {
                // Paid target — check if we need checkout (no payment or trial-eligible)
                let is_returning = org.base.trial_end_date.is_some()
                    || org.base.plan.as_ref().is_some_and(|p| !p.is_free());
                let is_trial_eligible = !is_returning && request.plan.config().trial_days > 0;
                let needs_checkout = !org.base.has_payment_method || is_trial_eligible;

                if needs_checkout {
                    // Route through Stripe Checkout to collect payment / apply trial
                    let cancel_url = request.url.clone();
                    let session = billing_service
                        .create_checkout_session(
                            organization_id,
                            request.plan,
                            success_url,
                            cancel_url,
                            auth.into_entity(),
                        )
                        .await?;
                    Ok(Json(ApiResponse::success(session.url.unwrap())))
                } else {
                    // Has payment, not trial-eligible — direct subscription update
                    let result = billing_service
                        .change_plan(organization_id, request.plan, auth.into_entity())
                        .await?;
                    Ok(Json(ApiResponse::success(result)))
                }
            }
        } else {
            // First-time subscriber — create Stripe checkout session
            let cancel_url = request.url.clone();

            let session = billing_service
                .create_checkout_session(
                    organization_id,
                    request.plan,
                    success_url,
                    cancel_url,
                    auth.into_entity(),
                )
                .await?;

            Ok(Json(ApiResponse::success(session.url.unwrap())))
        }
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Setup payment method (collect card without charging)
#[utoipa::path(
    post,
    path = "/setup-payment-method",
    tags = ["billing", "internal"],
    request_body = SetupPaymentMethodRequest,
    responses(
        (status = 200, description = "Setup session URL", body = ApiResponse<String>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn setup_payment_method(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(request): Json<SetupPaymentMethodRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let success_url = format!("{}?billing_flow=payment_setup", request.url);
    let cancel_url = request.url;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let session = billing_service
            .create_setup_payment_method_session(
                organization_id,
                success_url,
                cancel_url,
                auth.into_entity(),
            )
            .await?;

        Ok(Json(ApiResponse::success(session.url.unwrap())))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Change billing plan
///
/// Upgrades or downgrades the organization's billing plan.
#[utoipa::path(
    post,
    path = "/change-plan",
    tags = ["billing", "internal"],
    request_body = ChangePlanRequest,
    responses(
        (status = 200, description = "Plan change initiated", body = ApiResponse<String>),
        (status = 400, description = "Invalid plan or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn change_plan(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(request): Json<ChangePlanRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .change_plan(organization_id, request.plan, auth.into_entity())
            .await?;

        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Preview plan change (shows overage counts)
#[utoipa::path(
    get,
    path = "/change-plan/preview",
    tags = ["billing", "internal"],
    params(
        ("plan" = String, Query, description = "Target plan (JSON)"),
    ),
    responses(
        (status = 200, description = "Plan change preview", body = ApiResponse<ChangePlanPreview>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn preview_plan_change(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<ApiResponse<ChangePlanPreview>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    let plan_str = params
        .get("plan")
        .ok_or_else(|| ApiError::bad_request("Missing plan parameter"))?;

    let plan: BillingPlan =
        serde_json::from_str(plan_str).map_err(|_| ApiError::bad_request("Invalid plan format"))?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let preview = billing_service
            .preview_plan_change(organization_id, plan)
            .await?;

        Ok(Json(ApiResponse::success(preview)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Handle Stripe webhook
///
/// Internal endpoint for Stripe webhook callbacks.
#[utoipa::path(
    post,
    path = "/webhooks",
    tags = ["billing", "internal"],
    responses(
        (status = 200, description = "Webhook processed", body = EmptyApiResponse),
        (status = 400, description = "Invalid signature or billing not enabled", body = ApiErrorResponse),
    )
)]
async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> ApiResult<Json<ApiResponse<()>>> {
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            ApiError::validation(ErrorCode::ValidationRequired {
                field: "stripe-signature".to_string(),
            })
        })?;

    if let Some(billing_service) = &state.services.billing_service {
        billing_service.handle_webhook(&body, signature).await?;
        Ok(Json(ApiResponse::success(())))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Create a billing portal session
#[utoipa::path(
    post,
    path = "/portal",
    tags = ["billing", "internal"],
    request_body = String,
    responses(
        (status = 200, description = "Portal session URL", body = ApiResponse<String>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_portal_session(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(return_url): Json<String>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = &state.services.billing_service {
        let session_url = billing_service
            .create_portal_session(organization_id, return_url)
            .await?;

        Ok(Json(ApiResponse::success(session_url)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Submit enterprise plan inquiry
///
/// Updates Brevo contact/company with inquiry data, creates a deal, and
/// tracks an event for automation triggers. Requires authentication to
/// link the inquiry to an organization.
#[utoipa::path(
    post,
    path = "/inquiry",
    tags = ["billing", "internal"],
    request_body = EnterpriseInquiryRequest,
    responses(
        (status = 200, description = "Inquiry submitted successfully", body = EmptyApiResponse),
        (status = 400, description = "Invalid request or Brevo not configured", body = ApiErrorResponse),
        (status = 401, description = "Authentication required", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn submit_enterprise_inquiry(
    State(state): State<Arc<AppState>>,
    ClientIp(ip): ClientIp,
    auth: Authorized<Viewer>,
    Json(request): Json<EnterpriseInquiryRequest>,
) -> ApiResult<Json<ApiResponse<()>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if request.email.is_empty() || request.name.is_empty() || request.company.is_empty() {
        return Err(ApiError::validation(ErrorCode::ValidationRequired {
            field: "email, name, company".to_string(),
        }));
    }

    let brevo_service = state
        .services
        .brevo_service
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("Enterprise inquiries are not enabled"))?;

    // 1. Create a deal in the Sales pipeline
    let org = state
        .services
        .organization_service
        .get_by_id(&organization_id)
        .await?
        .ok_or_else(ApiError::organization_required)?;

    let deal_name = format!("Enterprise Inquiry - {}", &request.company);

    let company_ids = org.base.brevo_company_id.map(|id| vec![id]);
    let mut deal_attributes = serde_json::to_value(&request)?;
    if let Some(obj) = deal_attributes.as_object_mut() {
        obj.remove("email");
        obj.remove("company");
        obj.remove("name");
        // Brevo deal attributes must all be strings
        for value in obj.values_mut() {
            if let serde_json::Value::Number(n) = value {
                *value = serde_json::Value::String(n.to_string());
            }
        }
    }
    if let Err(e) = brevo_service
        .client
        .create_deal(&deal_name, Some(deal_attributes), company_ids)
        .await
    {
        tracing::warn!(
            error = %e,
            organization_id = %organization_id,
            "Failed to create Brevo deal for inquiry"
        );
    }

    // 2. Fire event for tracking and triggers
    if let Err(e) = brevo_service
        .client
        .track_event(
            "enterprise_inquiry",
            &request.email,
            Some(serde_json::to_value(&request)?),
        )
        .await
    {
        tracing::warn!(
            error = %e,
            "Failed to track enterprise_inquiry event in Brevo"
        );
    }

    tracing::info!(
        email = %request.email,
        company = %request.company,
        organization_id = %organization_id,
        plan_type = ?request.plan_type,
        client_ip = %ip,
        "Enterprise inquiry submitted to Brevo"
    );

    Ok(Json(ApiResponse::success(())))
}
