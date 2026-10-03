use crate::server::shared::types::api::ApiJson;
use axum::Json;
use axum::extract::{Path, State};
use std::sync::Arc;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

use crate::server::auth::middleware::permissions::{Authorized, Member};
use crate::server::bindings::r#impl::base::{Binding, BindingType};
use crate::server::bindings::service::BindingService;
use crate::server::config::AppState;
use crate::server::shared::handlers::query::BindingQuery;
use crate::server::shared::handlers::traits::{CrudHandlers, create_handler, update_handler};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Entity;
use crate::server::shared::types::api::{ApiError, ApiErrorResponse, ApiResponse, ApiResult};
use crate::server::shared::validation::validate_network_access;
impl CrudHandlers for Binding {
    type Service = BindingService;
    type FilterQuery = BindingQuery;
    type OrderField = crate::server::shared::handlers::ordering::NoOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.binding_service
    }
}

mod generated {
    use super::*;
    crate::crud_get_all_handler!(Binding);
    crate::crud_get_by_id_handler!(Binding);
    crate::crud_delete_handler!(Binding);
    crate::crud_bulk_delete_handler!(Binding);
    crate::crud_export_csv_handler!(Binding);
}

/// Validates that a binding doesn't conflict with existing bindings.
/// Rules:
/// - Interface binding conflicts with port bindings on same interface OR port bindings on all ip_addresses
/// - Port binding (specific ip_address) conflicts with interface binding on same interface
/// - Port binding (all ip_addresses) conflicts with ANY interface binding for this service
async fn validate_no_binding_type_conflict(
    state: &AppState,
    binding: &Binding,
    exclude_id: Option<Uuid>,
) -> Result<(), ApiError> {
    let service_id = binding.service_id();

    let filter = StorableFilter::<Binding>::new_from_network_ids(&[binding.base.network_id])
        .service_id(&service_id);
    let existing = state.services.binding_service.get_all(filter).await?;

    match binding.base.binding_type {
        BindingType::IPAddress { ip_address_id } => {
            // Check for conflicting port bindings: same interface OR all-interfaces
            for existing_binding in existing {
                if exclude_id == Some(existing_binding.id) {
                    continue;
                }

                // Conflict if port binding is on same interface OR on all ip_addresses
                if let BindingType::Port {
                    ip_address_id: existing_iface,
                    ..
                } = existing_binding.base.binding_type
                    && (existing_iface == Some(ip_address_id) || existing_iface.is_none())
                {
                    return Err(ApiError::conflict(
                        "Cannot add ip_address binding: service already has a port binding on this ip_address \
                             (or on all ip_addresses).",
                    ));
                }
            }
        }
        BindingType::Port {
            ip_address_id: Some(ip_address_id),
            ..
        } => {
            // Check for conflicting interface binding on same interface
            for existing_binding in existing {
                if exclude_id == Some(existing_binding.id) {
                    continue;
                }

                if let BindingType::IPAddress {
                    ip_address_id: existing_iface,
                } = existing_binding.base.binding_type
                    && existing_iface == ip_address_id
                {
                    return Err(ApiError::conflict(
                        "Cannot add port binding: service already has an ip_address binding on this ip_address.",
                    ));
                }
            }
        }
        BindingType::Port {
            ip_address_id: None,
            ..
        } => {
            // Port binding on all ip_addresses: conflicts with ANY interface binding
            for existing_binding in existing {
                if exclude_id == Some(existing_binding.id) {
                    continue;
                }

                if matches!(
                    existing_binding.base.binding_type,
                    BindingType::IPAddress { .. }
                ) {
                    return Err(ApiError::conflict(
                        "Cannot add port binding on all ip_addresses: service already has ip_address bindings.",
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Create a new Binding
///
/// Creates a binding that associates a service with a port or interface.
///
/// ### Binding Types
///
/// - **Interface binding**: Service is present at an interface (IP address) without a specific port.
///   Used for non-port-bound services like gateways.
/// - **Port binding (specific ip_address)**: Service listens on a specific port on a specific interface.
/// - **Port binding (all ip_addresses)**: Service listens on a specific port on all ip_addresses
///   (`ip_address_id: null`).
///
/// ### Validation and Deduplication Rules
///
/// - **Conflict detection**: Interface bindings conflict with port bindings on the same interface.
///   A port binding on all ip_addresses conflicts with any interface binding for the same service.
/// - **All-interfaces precedence**: When creating a port binding with `ip_address_id: null`,
///   any existing specific-interface bindings for the same port are automatically removed,
///   as they are superseded by the all-interfaces binding.
#[utoipa::path(
    post,
    path = "",
    tag = Binding::ENTITY_NAME_PLURAL,
    request_body = Binding,
    responses(
        (status = 200, description = "Binding created (superseded bindings may be removed)", body = ApiResponse<Binding>),
        (status = 400, description = "Referenced port or ip_address does not exist", body = ApiErrorResponse),
        (status = 409, description = "Conflict with existing binding type", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn create_binding(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    ApiJson(binding): ApiJson<Binding>,
) -> ApiResult<Json<ApiResponse<Binding>>> {
    // Guard tenancy before the supersede-delete below runs against a
    // caller-supplied network_id (create_handler only validates access at the
    // end, after the destructive deletes would already have fired).
    validate_network_access(Some(binding.network_id()), &auth.network_ids(), "access")?;

    validate_no_binding_type_conflict(&state, &binding, None).await?;

    // If creating an all-interfaces port binding, remove any specific-interface bindings for the same port
    // (the all-interfaces binding supersedes them)
    if let BindingType::Port {
        port_id,
        ip_address_id: None,
    } = &binding.base.binding_type
    {
        let service_id = binding.service_id();
        let filter = StorableFilter::<Binding>::new_from_network_ids(&[binding.network_id()])
            .service_id(&service_id);
        let existing = state.services.binding_service.get_all(filter).await?;

        for existing_binding in existing {
            if let BindingType::Port {
                port_id: existing_port_id,
                ip_address_id: Some(_),
            } = &existing_binding.base.binding_type
                && existing_port_id == port_id
            {
                // Delete the specific-interface binding that's being superseded
                tracing::info!(
                    binding_id = %existing_binding.id,
                    port_id = %existing_port_id,
                    "Removing specific-ip_address binding superseded by all-ip_addresses binding"
                );
                state
                    .services
                    .binding_service
                    .delete(&existing_binding.id, auth.entity.clone())
                    .await?;
            }
        }
    }

    create_handler::<Binding>(State(state), auth, ApiJson(binding)).await
}

/// Update a Binding
///
/// Updates an existing binding. The same conflict detection rules from binding creation apply.
///
/// ## Validation Rules
///
/// - **Conflict detection**: The updated binding must not conflict with other bindings on the
///   same service. Interface bindings conflict with port bindings on the same interface.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = Binding::ENTITY_NAME_PLURAL,
    params(("id" = Uuid, Path, description = "Binding ID")),
    request_body = Binding,
    responses(
        (status = 200, description = "Binding updated", body = ApiResponse<Binding>),
        (status = 400, description = "Referenced port or ip_address does not exist", body = ApiErrorResponse),
        (status = 409, description = "Conflict with existing binding type", body = ApiErrorResponse),
    ),
     security(("user_api_key" = []), ("session" = []))
)]
async fn update_binding(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Member>,
    path: Path<Uuid>,
    ApiJson(binding): ApiJson<Binding>,
) -> ApiResult<Json<ApiResponse<Binding>>> {
    // Guard tenancy before the conflict scan reads by the caller-supplied
    // network_id (update_handler validates access afterward).
    validate_network_access(Some(binding.network_id()), &auth.network_ids(), "access")?;

    validate_no_binding_type_conflict(&state, &binding, Some(*path)).await?;
    update_handler::<Binding>(State(state), auth, path, ApiJson(binding)).await
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(generated::get_all, create_binding))
        .routes(routes!(
            generated::get_by_id,
            update_binding,
            generated::delete
        ))
        .routes(routes!(generated::bulk_delete))
        .routes(routes!(generated::export_csv))
}
