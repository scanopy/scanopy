use crate::server::auth::middleware::permissions::{Authorized, Billing, Can, Read, Write};
use crate::server::billing::types::api::{
    CancelSubscriptionRequest, CancelSubscriptionResponse, ChangePlanPreview, ChangePlanRequest,
    CreateCheckoutRequest, FinalizePaymentMethodRequest, PauseSubscriptionRequest, SaveOfferCoupon,
    SetupIntentResponse,
};
use crate::server::billing::types::base::BillingPlan;
use crate::server::config::AppState;
use crate::server::openapi::tags as api_tags;
use crate::server::shared::types::ErrorCode;
use crate::server::shared::types::api::{ApiError, ApiJson, ApiResult};
use crate::server::shared::types::api::{ApiErrorResponse, ApiResponse, EmptyApiResponse};
use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::header::CACHE_CONTROL;
use axum::response::IntoResponse;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_billing_plans))
        .routes(routes!(create_checkout_session))
        .routes(routes!(create_payment_method_setup_intent))
        .routes(routes!(finalize_payment_method))
        .routes(routes!(change_plan))
        .routes(routes!(preview_plan_change))
        .routes(routes!(handle_webhook))
        .routes(routes!(create_portal_session))
        .routes(routes!(pause_subscription))
        .routes(routes!(resume_subscription))
        .routes(routes!(extend_trial))
        .routes(routes!(end_trial))
        .routes(routes!(cancel_subscription))
        .routes(routes!(reactivate_subscription))
        .routes(routes!(apply_discount_save_offer))
        .routes(routes!(get_save_offer_coupon))
        .merge(super::invoicing_handlers::create_router())
}

/// Get available billing plans
#[utoipa::path(
    get,
    path = "/plans",
    tags = [api_tags::INTERNAL],
    responses(
        (status = 200, description = "List of available billing plans", body = ApiResponse<Vec<BillingPlan>>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn get_billing_plans(
    State(state): State<Arc<AppState>>,
    _auth: Authorized<Can<Billing, Read>>,
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

/// Create checkout session
#[utoipa::path(
    post,
    path = "/checkout",
    tags = [api_tags::INTERNAL],
    request_body = CreateCheckoutRequest,
    responses(
        (status = 200, description = "Checkout session URL", body = ApiResponse<String>),
        (status = 400, description = "Invalid plan or billing not enabled", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_checkout_session(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Write>>,
    ApiJson(request): ApiJson<CreateCheckoutRequest>,
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

        // An air-gapped key carries the plan it was issued for and validates
        // offline, so a move to another license plan would leave a key on the
        // customer's server naming a plan they no longer hold, with no way to
        // retire it. Refused until the period they paid for ends; a cloud plan
        // and cancelling stay open. Checked before the routing below, because
        // the Checkout branches are plan changes too.
        if let Some(current_until) = org.air_gapped_plan_change_blocked_until(request.plan) {
            return Err(ApiError::air_gapped_plan_change_blocked(
                current_until.format("%B %-d, %Y").to_string(),
            ));
        }

        // Free is where a cloud org starts and where an ended cloud plan
        // lands; nobody picks it. Ending a subscription is Cancel Subscription.
        if request.plan.is_free() {
            return Err(ApiError::bad_request(
                "Free can't be chosen. Cancel your subscription to move to Free when it ends.",
            ));
        }

        let plan_status = org.base.plan_status;
        // A lapsed org has no live subscription to modify or schedule against:
        // its way back is a new paid subscription.
        let is_lapsed = org.is_lapsed();

        if plan_status.is_some() && org.base.stripe_customer_id.is_some() {
            {
                // Paid target — check trial eligibility and payment state
                use crate::server::billing::types::base::PlanStatus;
                let is_currently_trialing = plan_status == Some(PlanStatus::Trialing);

                if is_currently_trialing {
                    // Currently trialing — switch plan via subscription update (preserves trial)
                    let result = billing_service
                        .change_plan(organization_id, request.plan, auth.into_entity())
                        .await?;
                    return Ok(Json(ApiResponse::success(result)));
                }

                let has_non_free_plan = !org.base.plan.is_free();
                let is_returning = has_non_free_plan || org.base.trial_end_date.is_some();
                let is_trial_eligible = !is_returning && request.plan.config().trial_days > 0;

                // Authoritative card-on-file check via Stripe (not the
                // `has_payment_method` mirror, which lags the in-app SetupIntent
                // flow by an event-bus tick). Only queried when we might route to
                // a direct charge — trial-eligible orgs skip it.
                // A subscription billed by invoice needs no card to change plan.
                let has_payment_method = if is_trial_eligible {
                    false
                } else {
                    org.base.bills_by_invoice
                        || billing_service
                            .customer_has_payment_method(organization_id)
                            .await?
                };

                if is_trial_eligible {
                    // Trial-eligible — create subscription directly, skip Checkout
                    let result = billing_service
                        .create_trial_subscription(
                            organization_id,
                            request.plan,
                            auth.into_entity(),
                        )
                        .await?;
                    Ok(Json(ApiResponse::success(result)))
                } else if has_non_free_plan && has_payment_method && !is_lapsed {
                    // Live paid subscription + card on file — modify it in place.
                    let result = billing_service
                        .change_plan(organization_id, request.plan, auth.into_entity())
                        .await?;
                    Ok(Json(ApiResponse::success(result)))
                } else if has_payment_method {
                    // No live subscription to modify (currently on Free, or
                    // lapsed after a subscription ended), but a way to pay is
                    // on file — create the subscription here instead of
                    // handing the customer to Stripe Checkout. Routing such an
                    // org to change_plan would fail with "No active
                    // subscription found to modify"; it only updates an
                    // existing paid sub.
                    let result = billing_service
                        .create_paid_subscription(organization_id, request.plan, auth.into_entity())
                        .await?;
                    Ok(Json(ApiResponse::success(result)))
                } else {
                    // Nothing to pay with. Checkout collects a card and creates
                    // the subscription in one hosted step; the in-app dialog
                    // routes here only when it could not collect one itself.
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
            }
        } else {
            // No subscription: an org on Free, or a license signup whose
            // trial hasn't started. A trial is offered once per org.
            if request.plan.config().trial_days > 0 && org.base.trial_end_date.is_none() {
                // Trial plan — create subscription directly, skip Checkout. A
                // cloud plan's trial refuses without a card on file.
                let result = billing_service
                    .create_trial_subscription(organization_id, request.plan, auth.into_entity())
                    .await?;
                Ok(Json(ApiResponse::success(result)))
            } else if org.base.stripe_customer_id.is_some()
                && billing_service
                    .customer_has_payment_method(organization_id)
                    .await?
            {
                // Trial used, card on file — subscribe here rather than in Checkout.
                let result = billing_service
                    .create_paid_subscription(organization_id, request.plan, auth.into_entity())
                    .await?;
                Ok(Json(ApiResponse::success(result)))
            } else {
                // No trial — go through Stripe Checkout
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
        }
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Create payment method setup intent
///
/// A SetupIntent for collecting a card in the app with the Stripe Payment Element.
#[utoipa::path(
    post,
    path = "/payment-method-setup-intent",
    tags = [api_tags::INTERNAL],
    responses(
        (status = 200, description = "SetupIntent client secret", body = ApiResponse<SetupIntentResponse>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn create_payment_method_setup_intent(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Write>>,
) -> ApiResult<Json<ApiResponse<SetupIntentResponse>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let client_secret = billing_service
            .create_setup_intent(organization_id, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(SetupIntentResponse {
            client_secret,
        })))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Finalize payment method
///
/// Sets the card from a client-confirmed SetupIntent as the default.
#[utoipa::path(
    post,
    path = "/finalize-payment-method",
    tags = [api_tags::INTERNAL],
    request_body = FinalizePaymentMethodRequest,
    responses(
        (status = 200, description = "Payment method finalized", body = EmptyApiResponse),
        (status = 400, description = "Billing not enabled or SetupIntent invalid", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn finalize_payment_method(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Write>>,
    ApiJson(request): ApiJson<FinalizePaymentMethodRequest>,
) -> ApiResult<Json<EmptyApiResponse>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        billing_service
            .finalize_payment_method(organization_id, request.setup_intent_id, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(())))
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
    tags = [api_tags::INTERNAL],
    request_body = ChangePlanRequest,
    responses(
        (status = 200, description = "Plan change initiated", body = ApiResponse<String>),
        (status = 400, description = "Invalid plan or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn change_plan(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Write>>,
    ApiJson(request): ApiJson<ChangePlanRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        // Same rule as `create_checkout_session`: an air-gapped key commits the
        // org to its license tier until the period it paid for ends.
        let org = billing_service.get_organization(organization_id).await?;
        if let Some(current_until) = org.air_gapped_plan_change_blocked_until(request.plan) {
            return Err(ApiError::air_gapped_plan_change_blocked(
                current_until.format("%B %-d, %Y").to_string(),
            ));
        }

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
    tags = [api_tags::INTERNAL],
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
    auth: Authorized<Can<Billing, Read>>,
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
    tags = [api_tags::INTERNAL],
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

/// Create billing portal session
#[utoipa::path(
    post,
    path = "/portal",
    tags = [api_tags::INTERNAL],
    request_body = String,
    responses(
        (status = 200, description = "Portal session URL", body = ApiResponse<String>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_portal_session(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Write>>,
    ApiJson(return_url): ApiJson<String>,
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

/// Pause subscription billing
///
/// Pauses billing for a 30/60/90 day window. Eligibility: rolling 6-month
/// cooldown anchored on the org's `last_paused_at`.
#[utoipa::path(
    post,
    path = "/pause",
    tags = [api_tags::INTERNAL],
    request_body = PauseSubscriptionRequest,
    responses(
        (status = 200, description = "Subscription paused", body = ApiResponse<String>),
        (status = 400, description = "Ineligible or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn pause_subscription(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Write>>,
    ApiJson(request): ApiJson<PauseSubscriptionRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .pause_subscription(organization_id, request.duration_days, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Resume paused subscription
///
/// Clears Stripe pause collection and re-activates billing. Available while
/// `plan_status === 'paused'`. The prorated pause credit is posted to the
/// customer's Stripe balance asynchronously by the webhook arm that fires
/// for the `pause_collection` clear — the endpoint just returns success.
#[utoipa::path(
    post,
    path = "/resume",
    tags = [api_tags::INTERNAL],
    responses(
        (status = 200, description = "Subscription resumed", body = ApiResponse<String>),
        (status = 400, description = "No paused subscription or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn resume_subscription(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Write>>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .resume_subscription(organization_id, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Reactivate subscription pending cancellation
///
/// Clears Stripe's scheduled-cancellation state (`cancel_at` → None).
/// Available while `plan_status === 'pending_cancellation'`.
#[utoipa::path(
    post,
    path = "/reactivate",
    tags = [api_tags::INTERNAL],
    responses(
        (status = 200, description = "Subscription reactivated", body = ApiResponse<String>),
        (status = 400, description = "No pending cancellation or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn reactivate_subscription(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Write>>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .reactivate_subscription(organization_id, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Self-serve trial extend (+7 days, once per org lifetime)
#[utoipa::path(
    post,
    path = "/extend-trial",
    tags = [api_tags::INTERNAL],
    responses(
        (status = 200, description = "Trial extended", body = ApiResponse<String>),
        (status = 400, description = "Ineligible or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn extend_trial(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Write>>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .extend_trial(organization_id, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// End trial and charge card on file
///
/// Offered to customers who want an air-gapped license key, which needs a paid
/// subscription because it validates offline and cannot be revoked.
#[utoipa::path(
    post,
    path = "/end-trial",
    tags = [api_tags::INTERNAL],
    responses(
        (status = 200, description = "Trial ended and subscription charged", body = ApiResponse<String>),
        (status = 400, description = "Not trialing, no payment method, or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn end_trial(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Write>>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .end_trial(organization_id, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Cancel subscription
///
/// In-app cancel modal endpoint. Sets Stripe `cancel_at` to the current
/// period end (via Stripe's `MaxPeriodEnd` sentinel), stashes the canonical
/// Scanopy reason in subscription metadata, returns the period end so the
/// modal can render the retention disclosure.
#[utoipa::path(
    post,
    path = "/cancel",
    tags = [api_tags::INTERNAL],
    request_body = CancelSubscriptionRequest,
    responses(
        (status = 200, description = "Cancellation initiated", body = ApiResponse<CancelSubscriptionResponse>),
        (status = 400, description = "No active subscription or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn cancel_subscription(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Write>>,
    ApiJson(request): ApiJson<CancelSubscriptionRequest>,
) -> ApiResult<Json<ApiResponse<CancelSubscriptionResponse>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .cancel_subscription(organization_id, request, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Apply discount save offer
///
/// Applies the configured Stripe coupon to the subscription. Returns 400
/// when `STRIPE_SAVE_OFFER_COUPON_ID` is unset.
#[utoipa::path(
    post,
    path = "/cancel/apply-discount",
    tags = [api_tags::INTERNAL],
    responses(
        (status = 200, description = "Discount applied", body = ApiResponse<String>),
        (status = 400, description = "Discount not configured or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn apply_discount_save_offer(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Write>>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let result = billing_service
            .apply_discount_save_offer(organization_id, auth.into_entity())
            .await?;
        Ok(Json(ApiResponse::success(result)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}

/// Read live terms for the configured save-offer coupon
///
/// Returns the coupon's `percent_off` and `duration_in_months` so the
/// cancel modal's Discount panel can render the offer dynamically. The
/// payload is `null` when `STRIPE_SAVE_OFFER_COUPON_ID` is unset — the
/// modal hides the panel in that case.
#[utoipa::path(
    get,
    path = "/save-offer-coupon",
    tags = [api_tags::INTERNAL],
    responses(
        (status = 200, description = "Save-offer coupon terms, or null when not configured", body = ApiResponse<Option<SaveOfferCoupon>>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_save_offer_coupon(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Can<Billing, Read>>,
) -> ApiResult<Json<ApiResponse<Option<SaveOfferCoupon>>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;

    if let Some(billing_service) = state.services.billing_service.clone() {
        let coupon = billing_service
            .get_save_offer_coupon(organization_id)
            .await?;
        Ok(Json(ApiResponse::success(coupon)))
    } else {
        Err(ApiError::billing_setup_incomplete())
    }
}
