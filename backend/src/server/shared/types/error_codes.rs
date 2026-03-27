use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use strum::{EnumIter, IntoStaticStr};
use uuid::Uuid;

/// Helper macro to create a JSON map from key-value pairs.
macro_rules! json_map {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut map = serde_json::Map::new();
        $(
            map.insert($key.to_string(), serde_json::json!($value));
        )*
        map
    }};
}

/// Translatable error codes with optional parameters.
///
/// Each variant represents an error that can be displayed to users in their
/// preferred language. The frontend translates these codes using Paraglide.
///
/// Run `make generate-types` after modifying this enum to update:
/// - `ui/src/lib/generated/error-codes.ts` (TypeScript types)
/// - `ui/messages/en.json` (merged error messages)
#[derive(Debug, Clone, Serialize, Deserialize, EnumIter, IntoStaticStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ErrorCode {
    // === Validation ===
    /// A required field was not provided
    ValidationRequired { field: String },
    /// A field cannot be empty
    ValidationEmpty { field: String },
    /// Email address format is invalid
    ValidationInvalidEmail,
    /// IP address format is invalid
    ValidationInvalidIp,
    /// Field value is too short
    ValidationMinLength { field: String, min: u32 },
    /// Field value is too long
    ValidationMaxLength { field: String, max: u32 },
    /// Invalid format for a field
    ValidationInvalidFormat { field: String },
    /// No IDs provided for bulk operation
    ValidationBulkEmpty,

    // === Auth ===
    /// Email or password is incorrect
    AuthInvalidCredentials,
    /// Session has expired, user needs to log in again
    AuthSessionExpired,
    /// User lacks permission for this action
    AuthPermissionDenied,
    /// Operation requires an organization context
    AuthOrganizationRequired,
    /// Operation requires a user context
    AuthUserContextRequired,
    /// Operation requires an API key
    AuthApiKeyRequired,
    /// Operation requires a daemon context
    AuthDaemonRequired,
    /// Password is required
    AuthPasswordRequired,
    /// Password is invalid
    AuthPasswordInvalid,
    /// Not authenticated
    AuthNotAuthenticated,
    /// OIDC not configured for this organization
    AuthOidcNotConfigured,
    /// OIDC provider authentication failed
    AuthOidcProviderError { provider: String },
    /// User not found
    AuthUserNotFound { id: String },
    /// Daemon is trying to register with a key that has not yet been created (onboarding)
    AuthDaemonKeyNotCreated,
    /// Action is blocked in demo mode
    AuthDemoMode,
    /// Password login is disabled (OIDC-only mode)
    AuthPasswordLoginDisabled,
    /// User registration is disabled
    AuthRegistrationDisabled,
    /// Email verification required for this feature
    AuthEmailVerificationRequired,

    // === Generic Entity Operations ===
    /// Entity was not found
    EntityNotFound { entity: String, id: String },
    /// Entity with this name already exists
    EntityAlreadyExists { entity: String, name: String },
    /// Entity cannot be deleted because it's in use
    EntityInUse {
        entity: String,
        name: String,
        used_by: String,
    },
    /// Referenced entity does not exist
    EntityReferenceInvalid { entity: String, field: String },
    /// User doesn't have access to this entity
    EntityAccessDenied { entity: String, id: String },
    /// Entity has expired (shares, invites, api keys)
    EntityExpired { entity: String },
    /// Entity is disabled (shares, api keys)
    EntityDisabled { entity: String },
    /// At least one entity of this type is required
    EntityRequired { entity: String },
    /// Entity cannot be deleted
    EntityDeleteForbidden {
        entity: String,
        reason: Option<String>,
    },
    /// Entity cannot be updated
    EntityUpdateForbidden { entity: String },
    /// Entity is on a different network than expected
    EntityNetworkMismatch { entity: String },

    // === Hosts ===
    /// Host consolidation failed
    HostsConsolidateFailed { reason: String },

    // === Networks ===
    /// User doesn't have access to this network
    NetworksAccessDenied { network: String },

    // === Shares ===
    /// Password required for this share
    SharePasswordRequired,
    /// Incorrect password for share
    SharePasswordIncorrect,
    /// Domain not allowed for this share
    ShareDomainNotAllowed { domain: String },

    // === Invites ===
    /// Invite has already been accepted
    InviteAlreadyAccepted,
    /// Invite email doesn't match user's account
    InviteEmailMismatch,

    // === Discovery ===
    /// Historical discovery cannot be modified via API
    DiscoveryHistoricalReadOnly,
    /// Subnet is on a different network than the discovery
    DiscoverySubnetNetworkMismatch { subnet: String },
    /// Discovery session not found
    DiscoverySessionNotFound { id: Uuid },

    // === Interface ===
    /// IP address is not within subnet range
    InterfaceIpOutOfRange { ip: String, subnet: String },

    // === Daemon ===
    /// Cannot send updates for a different network
    DaemonNetworkMismatch,
    /// Cannot send updates for a different daemon
    DaemonIdentityMismatch,
    /// Daemon is on standby due to plan restrictions
    DaemonStandby,
    /// Daemon record not found — deleted or database was reset
    DaemonNotRegistered,
    /// Daemon version is older than the server version
    DaemonVersionTooOld {
        daemon_version: String,
        server_version: String,
    },

    // === User ===
    /// Email is already in use
    UserEmailInUse { email: String },

    // === Billing ===
    /// Payment is required to continue
    BillingPaymentRequired,
    /// Plan limit has been reached
    BillingPlanLimitReached { resource: String, limit: u32 },
    /// Active subscription required
    BillingSubscriptionRequired,
    /// Billing setup is incomplete
    BillingSetupIncomplete,
    /// Host limit reached on current plan
    BillingHostLimitReached { limit: u64 },
    /// Feature not available on current plan
    BillingFeatureNotAvailable { feature: String },

    // === Rate Limiting ===
    /// Too many requests
    RateLimitExceeded,

    // === External Services ===
    /// Error from external service
    ExternalServiceError { service: String, reason: String },

    // === Database ===
    /// Database operation failed
    DatabaseError,
    /// Unique constraint violation
    DatabaseDuplicateEntry { field: String },
}

impl ErrorCode {
    /// Returns the string code for this error (e.g., "validation_required")
    pub fn code(&self) -> &'static str {
        self.into()
    }

    /// Returns the default English message template.
    /// Parameters are denoted with {param_name} syntax.
    pub fn default_message(&self) -> &'static str {
        match self {
            // Validation
            Self::ValidationRequired { .. } => "Field '{field}' is required",
            Self::ValidationEmpty { .. } => "Field '{field}' cannot be empty",
            Self::ValidationInvalidEmail => "Invalid email address",
            Self::ValidationInvalidIp => "Invalid IP address format",
            Self::ValidationMinLength { .. } => "Field '{field}' must be at least {min} characters",
            Self::ValidationMaxLength { .. } => "Field '{field}' must be at most {max} characters",
            Self::ValidationInvalidFormat { .. } => "Invalid format for field '{field}'",
            Self::ValidationBulkEmpty => "No IDs provided for bulk operation",

            // Auth
            Self::AuthInvalidCredentials => "Invalid email or password",
            Self::AuthSessionExpired => "Your session has expired. Please log in again.",
            Self::AuthPermissionDenied => "You don't have permission to perform this action",
            Self::AuthOrganizationRequired => "This operation requires an organization context",
            Self::AuthUserContextRequired => "User context required",
            Self::AuthApiKeyRequired => "API key required",
            Self::AuthDaemonRequired => "Daemon context required",
            Self::AuthPasswordRequired => "Password required",
            Self::AuthPasswordInvalid => "Invalid password",
            Self::AuthNotAuthenticated => "Not authenticated",
            Self::AuthOidcNotConfigured => "OIDC not configured for this organization",
            Self::AuthOidcProviderError { .. } => "Failed to authenticate with {provider}",
            Self::AuthUserNotFound { .. } => "User with ID '{id}' not found",
            Self::AuthDaemonKeyNotCreated => {
                "Daemon is trying to register with an API key that has not yet been created"
            }
            Self::AuthDemoMode => "This action is disabled in demo mode",
            Self::AuthPasswordLoginDisabled => "Password login is disabled",
            Self::AuthRegistrationDisabled => "User registration is disabled",
            Self::AuthEmailVerificationRequired => {
                "Please verify your email to access this feature"
            }

            // Generic Entity Operations
            Self::EntityNotFound { .. } => "{entity} with ID '{id}' not found",
            Self::EntityAlreadyExists { .. } => "{entity} '{name}' already exists",
            Self::EntityInUse { .. } => {
                "Cannot delete {entity} '{name}' because it's used by {used_by}"
            }
            Self::EntityReferenceInvalid { .. } => {
                "Referenced {entity} in field '{field}' does not exist"
            }
            Self::EntityAccessDenied { .. } => "You don't have access to this {entity}",
            Self::EntityExpired { .. } => "This {entity} has expired",
            Self::EntityDisabled { .. } => "This {entity} is disabled",
            Self::EntityRequired { .. } => "At least one {entity} is required",
            Self::EntityDeleteForbidden { .. } => "Cannot delete this {entity}: {reason}",
            Self::EntityUpdateForbidden { .. } => "Cannot update this {entity}",
            Self::EntityNetworkMismatch { .. } => "{entity} is on a different network",

            // Hosts
            Self::HostsConsolidateFailed { .. } => "Failed to consolidate hosts: {reason}",

            // Networks
            Self::NetworksAccessDenied { .. } => "You don't have access to network '{network}'",

            // Shares
            Self::SharePasswordRequired => "Password required for this share",
            Self::SharePasswordIncorrect => "Incorrect password",
            Self::ShareDomainNotAllowed { .. } => "Domain '{domain}' not allowed",

            // Invites
            Self::InviteAlreadyAccepted => "This invite has already been accepted",
            Self::InviteEmailMismatch => "Invite email doesn't match your account",

            // Discovery
            Self::DiscoveryHistoricalReadOnly => "Historical discovery cannot be modified via API",
            Self::DiscoverySubnetNetworkMismatch { .. } => {
                "Subnet '{subnet}' is on a different network"
            }
            Self::DiscoverySessionNotFound { .. } => "Discovery session '{id}' not found",

            // Interface
            Self::InterfaceIpOutOfRange { .. } => {
                "IP address '{ip}' is not within subnet '{subnet}' range"
            }

            // Daemon
            Self::DaemonNetworkMismatch => "Cannot send updates for a different network",
            Self::DaemonIdentityMismatch => "Cannot send updates for a different daemon",
            Self::DaemonStandby => {
                "Your plan does not support DaemonPoll mode. The daemon is on standby. Upgrade your plan and restart the daemon to resume."
            }
            Self::DaemonNotRegistered => {
                "Daemon not found on server. It may have been deleted or the database was reset. Reinstall or reconfigure the daemon."
            }
            Self::DaemonVersionTooOld { .. } => {
                "Daemon version {daemon_version} is older than server version {server_version}. Update the daemon to match the server version."
            }

            // User
            Self::UserEmailInUse { .. } => "Email '{email}' is already in use",

            // Billing
            Self::BillingPaymentRequired => "Payment is required to continue",
            Self::BillingPlanLimitReached { .. } => {
                "You've reached the limit of {limit} {resource} on your current plan"
            }
            Self::BillingSubscriptionRequired => "Active subscription required",
            Self::BillingSetupIncomplete => "Billing setup is incomplete",
            Self::BillingHostLimitReached { .. } => {
                "You've reached the limit of {limit} hosts on your current plan. Upgrade for unlimited hosts."
            }
            Self::BillingFeatureNotAvailable { .. } => {
                "Your current plan does not include {feature}. Upgrade your plan to access this feature."
            }

            // Rate Limiting
            Self::RateLimitExceeded => "Too many requests, please try again later",

            // External Services
            Self::ExternalServiceError { .. } => "Error from {service}: {reason}",

            // Database
            Self::DatabaseError => "A database error occurred",
            Self::DatabaseDuplicateEntry { .. } => "A record with this {field} already exists",
        }
    }

    /// Returns the parameters for this error code as a JSON map.
    /// Returns None for error codes with no parameters.
    pub fn params(&self) -> Option<Map<String, Value>> {
        match self {
            // No params
            Self::ValidationInvalidEmail
            | Self::ValidationInvalidIp
            | Self::ValidationBulkEmpty
            | Self::AuthInvalidCredentials
            | Self::AuthSessionExpired
            | Self::AuthPermissionDenied
            | Self::AuthOrganizationRequired
            | Self::AuthUserContextRequired
            | Self::AuthApiKeyRequired
            | Self::AuthDaemonRequired
            | Self::AuthPasswordRequired
            | Self::AuthPasswordInvalid
            | Self::AuthNotAuthenticated
            | Self::AuthDaemonKeyNotCreated
            | Self::AuthDemoMode
            | Self::AuthPasswordLoginDisabled
            | Self::AuthRegistrationDisabled
            | Self::AuthEmailVerificationRequired
            | Self::AuthOidcNotConfigured
            | Self::SharePasswordRequired
            | Self::SharePasswordIncorrect
            | Self::InviteAlreadyAccepted
            | Self::InviteEmailMismatch
            | Self::DiscoveryHistoricalReadOnly
            | Self::DaemonNetworkMismatch
            | Self::DaemonIdentityMismatch
            | Self::DaemonStandby
            | Self::DaemonNotRegistered
            | Self::BillingPaymentRequired
            | Self::BillingSubscriptionRequired
            | Self::BillingSetupIncomplete
            | Self::RateLimitExceeded
            | Self::DatabaseError => None,

            // Validation with params
            Self::ValidationRequired { field }
            | Self::ValidationEmpty { field }
            | Self::ValidationInvalidFormat { field } => Some(json_map! { "field" => field }),
            Self::ValidationMinLength { field, min } => {
                Some(json_map! { "field" => field, "min" => min })
            }
            Self::ValidationMaxLength { field, max } => {
                Some(json_map! { "field" => field, "max" => max })
            }

            // Auth with params
            Self::AuthOidcProviderError { provider } => Some(json_map! { "provider" => provider }),
            Self::AuthUserNotFound { id } => Some(json_map! { "id" => id }),

            // Entity operations with params
            Self::EntityNotFound { entity, id } | Self::EntityAccessDenied { entity, id } => {
                Some(json_map! { "entity" => entity, "id" => id })
            }
            Self::EntityAlreadyExists { entity, name } => {
                Some(json_map! { "entity" => entity, "name" => name })
            }
            Self::EntityInUse {
                entity,
                name,
                used_by,
            } => Some(json_map! { "entity" => entity, "name" => name, "used_by" => used_by }),
            Self::EntityReferenceInvalid { entity, field } => {
                Some(json_map! { "entity" => entity, "field" => field })
            }
            Self::EntityExpired { entity }
            | Self::EntityDisabled { entity }
            | Self::EntityRequired { entity }
            | Self::EntityUpdateForbidden { entity }
            | Self::EntityNetworkMismatch { entity } => Some(json_map! { "entity" => entity }),
            Self::EntityDeleteForbidden { entity, reason } => Some(json_map! {
                "entity" => entity,
                "reason" => reason.as_deref().unwrap_or("")
            }),

            // Domain-specific with params
            Self::HostsConsolidateFailed { reason } => Some(json_map! { "reason" => reason }),
            Self::NetworksAccessDenied { network } => Some(json_map! { "network" => network }),
            Self::ShareDomainNotAllowed { domain } => Some(json_map! { "domain" => domain }),
            Self::DiscoverySubnetNetworkMismatch { subnet } => {
                Some(json_map! { "subnet" => subnet })
            }
            Self::DiscoverySessionNotFound { id } => Some(json_map! {"id" => id}),
            Self::InterfaceIpOutOfRange { ip, subnet } => {
                Some(json_map! { "ip" => ip, "subnet" => subnet })
            }
            Self::DaemonVersionTooOld {
                daemon_version,
                server_version,
            } => Some(
                json_map! { "daemon_version" => daemon_version, "server_version" => server_version },
            ),
            Self::UserEmailInUse { email } => Some(json_map! { "email" => email }),
            Self::BillingPlanLimitReached { resource, limit } => {
                Some(json_map! { "resource" => resource, "limit" => limit })
            }
            Self::BillingHostLimitReached { limit } => Some(json_map! { "limit" => limit }),
            Self::BillingFeatureNotAvailable { feature } => {
                Some(json_map! { "feature" => feature })
            }
            Self::ExternalServiceError { service, reason } => {
                Some(json_map! { "service" => service, "reason" => reason })
            }
            Self::DatabaseDuplicateEntry { field } => Some(json_map! { "field" => field }),
        }
    }

    /// Returns the message with parameters interpolated.
    /// Used as fallback for clients that don't support i18n.
    pub fn interpolated_message(&self) -> String {
        let mut message = self.default_message().to_string();

        if let Some(params) = self.params() {
            for (key, value) in params {
                let placeholder = format!("{{{}}}", key);
                let replacement = match value {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    _ => value.to_string(),
                };
                message = message.replace(&placeholder, &replacement);
            }
        }

        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_string() {
        assert_eq!(
            ErrorCode::ValidationInvalidEmail.code(),
            "validation_invalid_email"
        );
        assert_eq!(
            ErrorCode::EntityNotFound {
                entity: "Host".into(),
                id: "123".into()
            }
            .code(),
            "entity_not_found"
        );
    }

    #[test]
    fn test_interpolated_message() {
        let error = ErrorCode::EntityNotFound {
            entity: "Host".into(),
            id: "abc-123".into(),
        };
        assert_eq!(
            error.interpolated_message(),
            "Host with ID 'abc-123' not found"
        );
    }

    #[test]
    fn test_params() {
        let error = ErrorCode::ValidationMinLength {
            field: "password".into(),
            min: 8,
        };
        let params = error.params().unwrap();
        assert_eq!(params.get("field").unwrap(), "password");
        assert_eq!(params.get("min").unwrap(), 8);
    }

    #[test]
    fn test_no_params() {
        assert!(ErrorCode::AuthInvalidCredentials.params().is_none());
    }
}
