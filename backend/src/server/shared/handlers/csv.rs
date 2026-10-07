//! CSV export handler for entities
//!
//! Provides a generic handler for exporting entities to CSV format.
//! Uses the same filtering as list endpoints but ignores pagination to export all matching records.

use crate::server::{
    auth::middleware::permissions::{Authorized, Viewer},
    config::AppState,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, Entity as EntityEnum},
        extractors::Query,
        handlers::query::FilterQueryExtractor,
        services::{csv::build_csv, traits::CrudService},
        storage::filter::StorableFilter,
        types::api::{ApiError, ApiResult},
    },
};
use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use super::traits::CrudHandlers;

/// Export entities to CSV format.
///
/// Uses the same filtering as the list endpoint but ignores pagination parameters,
/// exporting ALL matching records.
pub async fn export_csv_handler<T>(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Viewer>,
    Query(query): Query<T::FilterQuery>,
) -> ApiResult<impl IntoResponse>
where
    T: CrudHandlers + 'static + ChangeTriggersTopologyStaleness<T> + Default,
    EntityEnum: From<T>,
{
    let site_ids = auth.site_ids();
    let organization_id = auth
        .organization_id()
        .ok_or_else(ApiError::organization_required)?;
    let user_id = auth.user_id();

    // Build base filter based on entity scoping (same as get_all_handler)
    let base_filter = StorableFilter::<T>::new_for_access(&site_ids, &organization_id);

    // Apply entity-specific filters (but NOT pagination - we want all records)
    let filter = query.apply_to_filter(base_filter, &site_ids, organization_id);

    let service = T::get_service(&state);

    // Get all matching records (no pagination)
    let entities = service.get_all(filter).await.map_err(|e| {
        tracing::error!(
            entity_type = T::table_name(),
            user_id = ?user_id,
            error = %e,
            "Failed to fetch entities for CSV export"
        );
        ApiError::internal_error(&e.to_string())
    })?;

    // Build CSV
    let csv_data = build_csv::<T>(&entities).map_err(|e| {
        tracing::error!(
            entity_type = T::table_name(),
            user_id = ?user_id,
            error = %e,
            "Failed to build CSV"
        );
        ApiError::internal_error(&format!("Failed to build CSV: {}", e))
    })?;

    // Build response with appropriate headers
    let filename = format!("{}.csv", T::entity_name_plural());
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/csv"));
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment; filename=\"export.csv\"")),
    );

    Ok((headers, Body::from(csv_data)))
}

/// Response type for CSV export (used in OpenAPI documentation)
pub struct CsvResponse(pub Response);

impl IntoResponse for CsvResponse {
    fn into_response(self) -> Response {
        self.0
    }
}
