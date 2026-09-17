//! Invoice billing endpoints for self-hosted plans: pay by sent invoice, open
//! and accept a quote, and keep the PO number current. Every endpoint reads
//! and writes only the caller's own Stripe customer.

use crate::server::auth::middleware::permissions::{Authorized, Owner};
use crate::server::billing::types::api::{
    AcceptQuoteRequest, InvoiceBillingRequest, InvoiceBillingStatus, UpdatePoNumberRequest,
};
use crate::server::config::AppState;
use crate::server::openapi::tags as api_tags;
use crate::server::shared::types::api::{
    ApiError, ApiErrorResponse, ApiResponse, ApiResult, EmptyApiResponse,
};
use axum::Json;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::IntoResponse;
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(set_up_invoice_billing))
        .routes(routes!(get_invoice_billing_status))
        .routes(routes!(cancel_quote))
        .routes(routes!(accept_quote))
        .routes(routes!(download_quote_pdf))
        .routes(routes!(update_po_number))
}

/// Pay by invoice
///
/// Records the billing entity on the organization's billing account, then
/// either issues the first invoice now or opens a quote for the buyer's
/// procurement to raise a purchase order against. Self-hosted plans only.
#[utoipa::path(
    post,
    path = "/invoice-billing",
    tags = [api_tags::BILLING, api_tags::INTERNAL],
    request_body = InvoiceBillingRequest,
    responses(
        (status = 200, description = "Invoice sent or quote opened", body = ApiResponse<String>),
        (status = 400, description = "Not a self-hosted plan, invalid details, or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn set_up_invoice_billing(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(request): Json<InvoiceBillingRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let billing_service = state
        .services
        .billing_service
        .clone()
        .ok_or_else(ApiError::billing_setup_incomplete)?;

    let result = billing_service
        .set_up_invoice_billing(
            organization_id,
            request.details,
            request.mode,
            request.plan,
            auth.into_entity(),
        )
        .await?;
    Ok(Json(ApiResponse::success(result)))
}

/// Invoice billing status
///
/// Whether the organization pays by invoice, the PO number on its invoices,
/// the unpaid invoice, and any open quote.
#[utoipa::path(
    get,
    path = "/invoice-billing",
    tags = [api_tags::BILLING, api_tags::INTERNAL],
    responses(
        (status = 200, description = "Invoice billing status", body = ApiResponse<InvoiceBillingStatus>),
        (status = 400, description = "Billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn get_invoice_billing_status(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
) -> ApiResult<Json<ApiResponse<InvoiceBillingStatus>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let billing_service = state
        .services
        .billing_service
        .clone()
        .ok_or_else(ApiError::billing_setup_incomplete)?;

    let status = billing_service
        .invoice_billing_status(organization_id)
        .await?;
    Ok(Json(ApiResponse::success(status)))
}

/// Cancel the open quote
#[utoipa::path(
    delete,
    path = "/quote",
    tags = [api_tags::BILLING, api_tags::INTERNAL],
    responses(
        (status = 200, description = "Quote cancelled", body = EmptyApiResponse),
        (status = 400, description = "No open quote or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn cancel_quote(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
) -> ApiResult<Json<EmptyApiResponse>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let billing_service = state
        .services
        .billing_service
        .clone()
        .ok_or_else(ApiError::billing_setup_incomplete)?;

    billing_service.cancel_quote(organization_id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// Accept the open quote
///
/// Stripe creates the invoiced subscription from the quote and sends the
/// first invoice, carrying the purchase order number if one is given.
#[utoipa::path(
    post,
    path = "/quote/accept",
    tags = [api_tags::BILLING, api_tags::INTERNAL],
    request_body = AcceptQuoteRequest,
    responses(
        (status = 200, description = "Quote accepted", body = ApiResponse<String>),
        (status = 400, description = "No open quote or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn accept_quote(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(request): Json<AcceptQuoteRequest>,
) -> ApiResult<Json<ApiResponse<String>>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let billing_service = state
        .services
        .billing_service
        .clone()
        .ok_or_else(ApiError::billing_setup_incomplete)?;

    let result = billing_service
        .accept_quote(organization_id, request.po_number)
        .await?;
    Ok(Json(ApiResponse::success(result)))
}

/// Download the open quote as a PDF
#[utoipa::path(
    get,
    path = "/quote/pdf",
    tags = [api_tags::BILLING, api_tags::INTERNAL],
    responses(
        (status = 200, description = "Quote PDF", content_type = "application/pdf", body = Vec<u8>),
        (status = 400, description = "No open quote or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn download_quote_pdf(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
) -> ApiResult<impl IntoResponse> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let billing_service = state
        .services
        .billing_service
        .clone()
        .ok_or_else(ApiError::billing_setup_incomplete)?;

    let (filename, bytes) = billing_service.quote_pdf(organization_id).await?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
        .map_err(|e| ApiError::internal_error(&e.to_string()))?;
    headers.insert(header::CONTENT_DISPOSITION, disposition);

    Ok((headers, Body::from(bytes)))
}

/// Update the PO number
///
/// Replaces the purchase order number printed on this organization's future
/// invoices, for a renewal raised against a new PO.
#[utoipa::path(
    put,
    path = "/po-number",
    tags = [api_tags::BILLING, api_tags::INTERNAL],
    request_body = UpdatePoNumberRequest,
    responses(
        (status = 200, description = "PO number updated", body = EmptyApiResponse),
        (status = 400, description = "Not a self-hosted plan or billing not enabled", body = ApiErrorResponse),
    ),
    security(("user_api_key" = []), ("session" = []))
)]
async fn update_po_number(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Json(request): Json<UpdatePoNumberRequest>,
) -> ApiResult<Json<EmptyApiResponse>> {
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let billing_service = state
        .services
        .billing_service
        .clone()
        .ok_or_else(ApiError::billing_setup_incomplete)?;

    billing_service
        .update_po_number(organization_id, request.po_number)
        .await?;
    Ok(Json(ApiResponse::success(())))
}
