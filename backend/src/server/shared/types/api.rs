use axum::{
    Json,
    extract::{FromRequest, Request, rejection::JsonRejection},
    http::StatusCode,
    response::Response,
};
use semver::Version;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use std::fmt;
use utoipa::ToSchema;
use uuid::Uuid;

pub type ApiResult<T> = Result<T, ApiError>;

const API_VERSION: u32 = 1;
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A validation error that should be returned as HTTP 400 Bad Request.
/// Use this for user-facing errors like invalid input, constraint violations, etc.
#[derive(Debug, Clone)]
pub struct ValidationError(pub String);

impl ValidationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ValidationError {}

impl From<ValidationError> for ApiError {
    fn from(err: ValidationError) -> Self {
        tracing::warn!("Validation error: {}", err.0);
        ApiError::bad_request(&err.0)
    }
}

/// Helper macro to return a validation error from a function returning anyhow::Result
#[macro_export]
macro_rules! bail_validation {
    ($($arg:tt)*) => {
        return Err($crate::server::shared::types::api::ValidationError::new(format!($($arg)*)).into())
    };
}

/// Pagination metadata returned with paginated responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationMeta {
    /// Total number of items matching the filter (ignoring pagination)
    pub total_count: u64,
    /// Maximum items per page (as requested)
    pub limit: u32,
    /// Number of items skipped
    pub offset: u32,
    /// Whether there are more items after this page
    pub has_more: bool,
}

impl PaginationMeta {
    /// Create pagination metadata from query results.
    pub fn new(total_count: u64, limit: u32, offset: u32) -> Self {
        let has_more = (offset as u64 + limit as u64) < total_count;
        Self {
            total_count,
            limit,
            offset,
            has_more,
        }
    }
}

fn server_version_example() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// API metadata included in all responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "api_version": API_VERSION,
    "server_version": SERVER_VERSION
}))]
pub struct ApiMeta {
    /// API version (integer, increments on breaking changes)
    pub api_version: u32,
    /// Server version (semver)
    #[schema(value_type = String, example = server_version_example)]
    pub server_version: Version,
}

impl Default for ApiMeta {
    fn default() -> Self {
        Self {
            api_version: API_VERSION,
            server_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
        }
    }
}

/// API metadata for paginated list responses (pagination is always present)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "api_version": API_VERSION,
    "server_version": SERVER_VERSION,
    "pagination": {
        "total_count": 142,
        "limit": 50,
        "offset": 0,
        "has_more": true
    }
}))]
pub struct PaginatedApiMeta {
    /// API version (integer, increments on breaking changes)
    pub api_version: u32,
    /// Server version (semver)
    #[schema(value_type = String, example = server_version_example)]
    pub server_version: Version,
    /// Pagination info
    pub pagination: PaginationMeta,
}

impl PaginatedApiMeta {
    pub fn new(total_count: u64, limit: u32, offset: u32) -> Self {
        Self {
            api_version: API_VERSION,
            server_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            pagination: PaginationMeta::new(total_count, limit, offset),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub meta: ApiMeta,
}

pub type EmptyApiResponse = ApiResponse<()>;

/// Error response type for API errors (no data field)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiErrorResponse {
    pub success: bool,
    pub error: Option<String>,
    /// Machine-readable error code for i18n translation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Parameters for interpolating into the translated error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Map<String, serde_json::Value>>,
    /// API metadata (version info)
    pub meta: ApiMeta,
}

impl ApiErrorResponse {
    /// Check if this response matches the error code from an ApiError
    pub fn matches_error(&self, expected: &ApiError) -> bool {
        match (&self.code, &expected.error_code) {
            (Some(response_code), Some(expected_code)) => response_code == expected_code.code(),
            _ => false,
        }
    }
}

impl fmt::Display for ApiErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error.as_deref().unwrap_or("Unknown error"))
    }
}

impl std::error::Error for ApiErrorResponse {}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            meta: ApiMeta::default(),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            meta: ApiMeta::default(),
        }
    }
}

/// Response type for paginated list endpoints (pagination is always present in meta)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedApiResponse<T> {
    pub success: bool,
    pub data: Vec<T>,
    pub meta: PaginatedApiMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> PaginatedApiResponse<T> {
    pub fn success(data: Vec<T>, total_count: u64, limit: u32, offset: u32) -> Self {
        Self {
            success: true,
            data,
            error: None,
            meta: PaginatedApiMeta::new(total_count, limit, offset),
        }
    }
}

use crate::server::shared::storage::traits::Entity;

use super::error_codes::ErrorCode;

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
    /// Optional error code for i18n translation on the frontend
    pub error_code: Option<ErrorCode>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ApiError {}

impl ApiError {
    pub fn new(status: StatusCode, message: String) -> Self {
        Self {
            status,
            message,
            error_code: None,
        }
    }

    /// Create an error with a translatable error code.
    /// The message is auto-generated from the error code for non-i18n clients.
    pub fn coded(status: StatusCode, code: ErrorCode) -> Self {
        Self {
            status,
            message: code.interpolated_message(),
            error_code: Some(code),
        }
    }

    /// Validation error (400) with a translatable error code
    pub fn validation(code: ErrorCode) -> Self {
        Self::coded(StatusCode::BAD_REQUEST, code)
    }

    /// Not found error (404) for an entity
    pub fn entity_not_found<T: Entity>(id: impl ToString) -> Self {
        Self::coded(
            StatusCode::NOT_FOUND,
            ErrorCode::EntityNotFound {
                entity: T::entity_name_singular().to_string(),
                id: id.to_string(),
            },
        )
    }

    /// Entity already exists (400)
    pub fn entity_exists<T: Entity>(name: impl ToString) -> Self {
        Self::coded(
            StatusCode::BAD_REQUEST,
            ErrorCode::EntityAlreadyExists {
                entity: T::entity_name_singular().to_string(),
                name: name.to_string(),
            },
        )
    }

    pub fn conflict(message: &str) -> Self {
        Self::new(StatusCode::CONFLICT, message.to_string())
    }

    pub fn forbidden(message: &str) -> Self {
        Self::new(StatusCode::FORBIDDEN, message.to_string())
    }

    /// Forbidden error (403) with organization required code
    pub fn organization_required() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::AuthOrganizationRequired)
    }

    /// Forbidden error (403) with permission denied code
    pub fn permission_denied() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::AuthPermissionDenied)
    }

    pub fn internal_error(message: &str) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message.to_string())
    }

    pub fn bad_request(message: &str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message.to_string())
    }

    pub fn not_found(message: String) -> Self {
        Self::new(StatusCode::NOT_FOUND, message.to_string())
    }

    pub fn unauthorized(message: String) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message.to_string())
    }

    /// Unauthorized error (401) with invalid credentials code
    pub fn invalid_credentials() -> Self {
        Self::coded(StatusCode::UNAUTHORIZED, ErrorCode::AuthInvalidCredentials)
    }

    /// Unauthorized error (401) with session expired code
    pub fn session_expired() -> Self {
        Self::coded(StatusCode::UNAUTHORIZED, ErrorCode::AuthSessionExpired)
    }

    pub fn bad_gateway(message: String) -> Self {
        Self::new(StatusCode::BAD_GATEWAY, message.to_string())
    }

    pub fn too_many_requests(message: String) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, message.to_string())
    }

    pub fn payment_required(message: &str) -> Self {
        Self::new(StatusCode::PAYMENT_REQUIRED, message.to_string())
    }

    /// Payment required (402) with billing code
    pub fn billing_required() -> Self {
        Self::coded(
            StatusCode::PAYMENT_REQUIRED,
            ErrorCode::BillingPaymentRequired,
        )
    }

    /// Bad request (400) - billing setup is incomplete
    pub fn billing_setup_incomplete() -> Self {
        Self::coded(StatusCode::BAD_REQUEST, ErrorCode::BillingSetupIncomplete)
    }

    // === Auth errors ===

    /// Forbidden (403) - user context required
    pub fn user_required() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::AuthUserContextRequired)
    }

    /// Forbidden (403) - API key required
    pub fn api_key_required() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::AuthApiKeyRequired)
    }

    /// Forbidden (403) - daemon context required
    pub fn daemon_required() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::AuthDaemonRequired)
    }

    /// Forbidden (403) - password required
    pub fn password_required() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::AuthPasswordRequired)
    }

    /// Bad request (400) - password invalid
    pub fn password_invalid() -> Self {
        Self::coded(StatusCode::BAD_REQUEST, ErrorCode::AuthPasswordInvalid)
    }

    /// Unauthorized (401) - not authenticated
    pub fn not_authenticated() -> Self {
        Self::coded(StatusCode::UNAUTHORIZED, ErrorCode::AuthNotAuthenticated)
    }

    /// Unauthorized (401) - not authenticated
    pub fn daemon_key_not_yet_active() -> Self {
        Self::coded(StatusCode::UNAUTHORIZED, ErrorCode::AuthDaemonKeyNotCreated)
    }

    /// Forbidden (403) - action blocked in demo mode
    pub fn demo_mode_blocked() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::AuthDemoMode)
    }

    // === Generic entity operations ===

    /// Forbidden (403) - access denied to entity
    pub fn entity_access_denied<T: Entity>(id: impl ToString) -> Self {
        Self::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::EntityAccessDenied {
                entity: T::entity_name_singular().to_string(),
                id: id.to_string(),
            },
        )
    }

    /// Forbidden (403) - entity has expired
    pub fn entity_expired<T: Entity>() -> Self {
        Self::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::EntityExpired {
                entity: T::entity_name_singular().to_string(),
            },
        )
    }

    /// Forbidden (403) - entity is disabled
    pub fn entity_disabled<T: Entity>() -> Self {
        Self::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::EntityDisabled {
                entity: T::entity_name_singular().to_string(),
            },
        )
    }

    /// Forbidden (403) - daemon API key has expired.
    pub fn daemon_api_key_expired() -> Self {
        use crate::server::daemon_api_keys::r#impl::base::DaemonApiKey;
        Self::entity_expired::<DaemonApiKey>()
    }

    /// Forbidden (403) - daemon API key is disabled.
    pub fn daemon_api_key_disabled() -> Self {
        use crate::server::daemon_api_keys::r#impl::base::DaemonApiKey;
        Self::entity_disabled::<DaemonApiKey>()
    }

    /// Bad request (400) - at least one entity required
    pub fn entity_required<T: Entity>() -> Self {
        Self::coded(
            StatusCode::BAD_REQUEST,
            ErrorCode::EntityRequired {
                entity: T::entity_name_singular().to_string(),
            },
        )
    }

    /// Bad request (400) - entity is on a different network
    pub fn entity_network_mismatch<T: Entity>() -> Self {
        Self::coded(
            StatusCode::BAD_REQUEST,
            ErrorCode::EntityNetworkMismatch {
                entity: T::entity_name_singular().to_string(),
            },
        )
    }

    /// Forbidden (403) - entity cannot be deleted
    pub fn entity_delete_forbidden<T: Entity>(reason: Option<&str>) -> Self {
        Self::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::EntityDeleteForbidden {
                entity: T::entity_name_singular().to_string(),
                reason: reason.map(|r| r.to_string()),
            },
        )
    }

    /// Forbidden (403) - entity cannot be updated
    pub fn entity_update_forbidden<T: Entity>() -> Self {
        Self::coded(
            StatusCode::FORBIDDEN,
            ErrorCode::EntityUpdateForbidden {
                entity: T::entity_name_singular().to_string(),
            },
        )
    }

    // === Validation errors ===

    /// Bad request (400) - field cannot be empty
    pub fn field_empty(field: &str) -> Self {
        Self::coded(
            StatusCode::BAD_REQUEST,
            ErrorCode::ValidationEmpty {
                field: field.to_string(),
            },
        )
    }

    /// Bad request (400) - no IDs provided for bulk operation
    pub fn bulk_empty() -> Self {
        Self::coded(StatusCode::BAD_REQUEST, ErrorCode::ValidationBulkEmpty)
    }

    // === Interface errors ===

    /// Bad request (400) - IP address is not within subnet range
    pub fn interface_ip_out_of_range(ip: &str, subnet: &str) -> Self {
        Self::coded(
            StatusCode::BAD_REQUEST,
            ErrorCode::InterfaceIpOutOfRange {
                ip: ip.to_string(),
                subnet: subnet.to_string(),
            },
        )
    }

    // === Share errors ===

    /// Forbidden (403) - password required for share
    pub fn share_password_required() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::SharePasswordRequired)
    }

    /// Forbidden (403) - incorrect share password
    pub fn share_password_incorrect() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::SharePasswordIncorrect)
    }

    // === Invite errors ===

    /// Forbidden (403) - invite already accepted
    pub fn invite_already_accepted() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::InviteAlreadyAccepted)
    }

    /// Forbidden (403) - invite email mismatch
    pub fn invite_email_mismatch() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::InviteEmailMismatch)
    }

    /// Forbidden (429) - rate limit exceeded
    pub fn rate_limit_exceeded() -> Self {
        Self::coded(StatusCode::TOO_MANY_REQUESTS, ErrorCode::RateLimitExceeded)
    }

    // === Discovery errors ===

    pub fn discovery_session_not_found(id: Uuid) -> Self {
        Self::coded(
            StatusCode::NOT_FOUND,
            ErrorCode::DiscoverySessionNotFound { id },
        )
    }

    /// Bad request (400) - historical discovery is read-only
    pub fn discovery_historical_read_only() -> Self {
        Self::coded(
            StatusCode::BAD_REQUEST,
            ErrorCode::DiscoveryHistoricalReadOnly,
        )
    }

    /// Bad request (400) - subnet is on a different network than the discovery
    pub fn discovery_subnet_network_mismatch(subnet: &str) -> Self {
        Self::coded(
            StatusCode::BAD_REQUEST,
            ErrorCode::DiscoverySubnetNetworkMismatch {
                subnet: subnet.to_string(),
            },
        )
    }

    // === Daemon errors ===

    /// Forbidden (403) - daemon cannot send updates for a different network
    pub fn daemon_network_mismatch() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::DaemonNetworkMismatch)
    }

    /// Forbidden (403) - daemon cannot send updates for a different daemon
    pub fn daemon_identity_mismatch() -> Self {
        Self::coded(StatusCode::FORBIDDEN, ErrorCode::DaemonIdentityMismatch)
    }

    /// Bad request (400) - daemon version is older than server version
    pub fn daemon_version_too_old(daemon_version: &str, server_version: &str) -> Self {
        Self::coded(
            StatusCode::BAD_REQUEST,
            ErrorCode::DaemonVersionTooOld {
                daemon_version: daemon_version.to_string(),
                server_version: server_version.to_string(),
            },
        )
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (code, params) = if let Some(ref error_code) = self.error_code {
            (Some(error_code.code().to_string()), error_code.params())
        } else {
            (None, None)
        };

        let response = ApiErrorResponse {
            success: false,
            error: Some(self.message),
            code,
            params,
            meta: ApiMeta::default(),
        };
        (self.status, Json(response)).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        // Check if this is an ApiError (preserves status code and error code)
        if let Some(api_err) = err.downcast_ref::<ApiError>() {
            return api_err.clone();
        }

        // Check if this is a ValidationError (should return 400)
        if let Some(validation_err) = err.downcast_ref::<ValidationError>() {
            tracing::warn!("Validation error: {}", validation_err.0);
            return Self::bad_request(&validation_err.0);
        }

        // All other anyhow errors are internal server errors
        let msg = err.to_string();
        tracing::error!("Internal error: {}", msg);
        Self::internal_error(&msg)
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => {
                tracing::warn!("Database error: row not found");
                Self::not_found("Row not found".to_string())
            }
            sqlx::Error::Database(db_err) => {
                // Check for constraint violations that indicate user error (400)
                if db_err.is_foreign_key_violation() {
                    tracing::warn!("Database error: foreign key violation - {}", db_err);
                    return Self::bad_request("Referenced entity does not exist");
                }
                if db_err.is_unique_violation() {
                    tracing::warn!("Database error: unique constraint violation - {}", db_err);
                    return Self::bad_request("Entity already exists");
                }
                if db_err.is_check_violation() {
                    tracing::warn!("Database error: check constraint violation - {}", db_err);
                    return Self::bad_request("Invalid data");
                }
                // Other database errors are internal
                tracing::error!("Database error: {}", db_err);
                Self::internal_error("Database operation failed")
            }
            _ => {
                tracing::error!("Database error: {}", err);
                Self::internal_error("Database operation failed")
            }
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        tracing::error!("JSON serialization error: {}", err);
        Self::bad_request("Invalid JSON data")
    }
}

/// Custom JSON extractor that returns ApiError on rejection.
/// This ensures deserialization errors are returned in our standard API format.
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(ApiJson(value)),
            Err(rejection) => {
                let message = rejection.body_text();
                tracing::warn!("JSON deserialization failed: {}", message);
                // Extract the useful part of the error message
                let friendly_message = if message.contains("Failed to deserialize") {
                    // Extract the actual error after the boilerplate
                    message.split(": ").skip(1).collect::<Vec<_>>().join(": ")
                } else {
                    message
                };
                Err(ApiError::bad_request(&friendly_message))
            }
        }
    }
}

pub trait EmptyToOption<T> {
    fn empty_to_option(self) -> Option<T>;
}

// Implement for common types that can be "empty"
impl EmptyToOption<String> for String {
    fn empty_to_option(self) -> Option<String> {
        if self.is_empty() { None } else { Some(self) }
    }
}

impl EmptyToOption<String> for Option<String> {
    fn empty_to_option(self) -> Option<String> {
        match self {
            Some(s) if s.is_empty() => None,
            other => other,
        }
    }
}

impl<T> EmptyToOption<Vec<T>> for Vec<T> {
    fn empty_to_option(self) -> Option<Vec<T>> {
        if self.is_empty() { None } else { Some(self) }
    }
}

pub fn serialize_sensitive_info<S>(_key: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str("**********")
}

pub fn serialize_optional_sensitive_info<S>(
    _key: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str("**********")
}

pub fn deserialize_empty_string_as_none<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.and_then(|s| if s.is_empty() { None } else { Some(s) }))
}

pub fn deserialize_empty_vec_as_none<'de, D, T>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    let opt = Option::<Vec<T>>::deserialize(deserializer)?;
    Ok(opt.and_then(|vec| if vec.is_empty() { None } else { Some(vec) }))
}
