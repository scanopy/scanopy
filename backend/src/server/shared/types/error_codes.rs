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
    /// Email domain cannot receive mail (no MX/A/AAAA DNS records)
    ValidationEmailDomainUndeliverable,
    /// Email validation rated the address unlikely to receive mail, or randomly generated
    ValidationEmailUndeliverable,
    /// Email validation identified the address as a disposable inbox
    ValidationEmailDisposable,
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
    /// A modern daemon (>= server-provisioned-identity floor) tried to self-register but
    /// isn't provisioned. It must be provisioned in the UI first; this is a terminal,
    /// non-retryable rejection (the server is reachable and answered definitively).
    AuthDaemonNotProvisioned,
    /// Action is blocked in demo mode
    AuthDemoMode,
    /// Password login is disabled (OIDC-only mode)
    AuthPasswordLoginDisabled,
    /// User registration is disabled
    AuthRegistrationDisabled,
    /// Self-hosted instance already has its single organization; further
    /// non-invite sign-ups must join the existing org via an invite.
    AuthOrgLimitReached,
    /// Email verification required for this feature
    AuthEmailVerificationRequired,
    /// The user accepted an earlier version of the terms, or none, and must
    /// accept the current terms before using the app
    AuthTermsAcceptanceRequired,
    /// Email verification link is unknown, already used, or expired
    AuthVerificationTokenInvalid,
    /// A verification link went to this address too recently to send another
    AuthVerificationEmailCooldown { seconds: u64 },

    // === License ===
    /// Server is in read-only mode due to license issue
    LicenseLocked,
    /// The online key already runs on as many servers as the plan covers
    LicenseInstanceLimitReached { active: u32, limit: u32 },
    /// The organization's plan is not a self-hosted license plan, so it has no key
    LicensePlanHasNoKey,

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
    /// Entity is on a different site than expected
    EntitySiteMismatch { entity: String },

    // === Hosts ===
    /// Host merge failed
    HostsMergeFailed { reason: String },

    // === Sites ===
    /// User doesn't have access to this site
    SitesAccessDenied { site: String },

    // === Shares ===
    /// Password required for this share
    SharePasswordRequired,
    /// Incorrect password for share
    SharePasswordIncorrect,
    /// Share access token is invalid, expired, or signed with a stale password hash
    ShareTokenInvalid,

    // === Invites ===
    /// Invite has already been accepted
    InviteAlreadyAccepted,
    /// Invite email doesn't match user's account
    InviteEmailMismatch,

    // === Email ===
    /// The configured mail server refused the message. Deliberately says nothing about
    /// why: the mail server's reply names the relay host and carries provider
    /// diagnostics, which belong in the server log and not in front of an end user.
    EmailDeliveryFailed,
    /// An SNS delivery to the SES feedback endpoint that isn't an authentic message for the
    /// configured topic: a bad signature, or a topic ARN that doesn't match
    /// `SCANOPY_EMAIL_FEEDBACK_SNS_TOPIC_ARN`.
    EmailFeedbackRejected,

    // === Credentials ===
    /// A credential identical to the organization's default for its type, which already exists
    CredentialDefaultExists { name: String },
    /// The organization's default credential for a type cannot be deleted
    CredentialDefaultDeleteForbidden { name: String },
    /// The settings of an organization's default credential cannot change
    CredentialDefaultSettingsLocked { name: String },
    /// A credential that runs over an SSH login was saved without one
    CredentialLoginRequired,
    /// The login a credential names is missing, in another organization, or not a login
    CredentialLoginInvalid,
    /// A credential that runs over no login named one
    CredentialLoginNotUsed,
    /// A credential and its login name different daemon OSes
    CredentialLoginDaemonOsMismatch { login: String },
    /// A login is still used by other credentials, so it cannot be deleted
    CredentialLoginInUse { name: String, users: String },
    /// A login cannot be assigned to sites or hosts: the credentials that use it are
    CredentialLoginNotAssignable { name: String },

    // === Discovery ===
    /// Historical discovery cannot be modified via API
    DiscoveryHistoricalReadOnly,
    /// Subnet is on a different site than the discovery
    DiscoverySubnetSiteMismatch { subnet: String },
    /// Discovery session not found
    DiscoverySessionNotFound { id: Uuid },

    // === IP address ===
    /// IP address is not within subnet range
    IpAddressOutOfRange { ip: String, subnet: String },

    // === Subnet ===
    /// Internet and Remote Network ranges are seeded for every site and can't be created
    SubnetTypeSeededPerSite { subnet_type: String },

    // === Daemon ===
    /// Cannot send updates for a different site
    DaemonSiteMismatch,
    /// Cannot send updates for a different daemon
    DaemonIdentityMismatch,
    /// Daemon is on standby due to plan restrictions
    DaemonStandby,
    /// Daemon record not found — deleted or database was reset
    DaemonNotRegistered,
    /// The plan runs one daemon, and the organization already has it
    DaemonPlanRequired,
    /// Daemon version is at or below the last version this server rejects
    DaemonVersionTooOld {
        daemon_version: String,
        last_unsupported_version: String,
    },
    /// Daemon runs on a different OS from the one it was created for
    DaemonOsMismatch { expected: String, actual: String },

    // === User ===
    /// Email is already in use
    UserEmailInUse { email: String },

    // === Organizations ===
    /// Organization cannot be deleted while it has an active paid subscription
    OrganizationHasActiveSubscription,

    // === Billing ===
    /// Payment is required to continue
    BillingPaymentRequired,
    /// Plan limit has been reached
    BillingPlanLimitReached { resource: String, limit: u32 },
    /// Active subscription required
    BillingSubscriptionRequired,
    /// Billing setup is incomplete
    BillingSetupIncomplete,
    /// A Stripe webhook delivery whose signature doesn't verify against the webhook secret
    BillingWebhookSignatureInvalid,
    /// Host limit reached on current plan
    BillingHostLimitReached { limit: u64 },
    /// Feature not available on current plan
    BillingFeatureNotAvailable { feature: String },
    /// Organization is on a self-hosted plan, so the cloud app's main routes
    /// are closed to it
    BillingSelfHostedPlanLocked,
    /// The org's subscription ended and it has not chosen a paid plan since,
    /// so the app is read-only for it
    BillingPlanLapsed,
    /// A cloud org has not chosen a plan yet, so only the Settings routes
    /// are open to it
    BillingPlanRequired,
    /// Cannot leave an air-gapped key until the current licence period ends
    BillingAirGappedKeyStillCurrent,
    /// Cannot change plan while an air-gapped key is still current. The key
    /// carries the plan it was issued for and validates offline, so the org is
    /// committed until the period it paid for ends. Cancelling stays open.
    BillingAirGappedPlanChangeBlocked { date: String },

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
            Self::ValidationEmailDomainUndeliverable => {
                "This email domain can't receive mail. Please use a valid email address."
            }
            Self::ValidationEmailUndeliverable => {
                "We couldn't confirm this address can receive mail. Check it for typos or use a different address."
            }
            Self::ValidationEmailDisposable => {
                "Temporary email addresses can't be used. Please sign up with a permanent address."
            }
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
            Self::AuthDaemonNotProvisioned => {
                "This daemon must be provisioned before it can register. Provision it in the Scanopy UI and re-run the install command."
            }
            Self::AuthDemoMode => "This action is disabled in demo mode",
            Self::LicenseLocked => "Your license is not active. The server is in read-only mode.",
            Self::LicensePlanHasNoKey => {
                "This organization's plan has no license key. License keys come with the Self-Hosted Standard and Plus plans."
            }
            Self::LicenseInstanceLimitReached { .. } => {
                "This license key is in use on {active} servers and your plan covers {limit}, so this server is in read-only mode. Delete a server you no longer run under License in Scanopy Cloud, then restart this one."
            }
            Self::AuthPasswordLoginDisabled => "Password login is disabled",
            Self::AuthRegistrationDisabled => "User registration is disabled",
            Self::AuthOrgLimitReached => {
                "This server has reached its maximum number of organizations. Ask an admin to invite you to an existing organization."
            }
            Self::AuthEmailVerificationRequired => {
                "Please verify your email to access this feature"
            }
            Self::AuthTermsAcceptanceRequired => {
                "Please accept the updated terms of service to continue"
            }
            Self::AuthVerificationTokenInvalid => {
                "This verification link is invalid or has expired. Request a new one."
            }
            Self::AuthVerificationEmailCooldown { .. } => {
                "A link was just sent to this address. Try again in {seconds} seconds."
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
            Self::EntitySiteMismatch { .. } => "{entity} is on a different site",

            // Hosts
            Self::HostsMergeFailed { .. } => "Failed to merge hosts: {reason}",

            // Sites
            Self::SitesAccessDenied { .. } => "You don't have access to site '{site}'",

            // Shares
            Self::SharePasswordRequired => "Password required for this share",
            Self::SharePasswordIncorrect => "Incorrect password",
            Self::ShareTokenInvalid => "Access token is invalid or expired",

            // Invites
            Self::InviteAlreadyAccepted => "This invite has already been accepted",
            Self::InviteEmailMismatch => "Invite email doesn't match your account",

            // Email
            Self::EmailDeliveryFailed => {
                "Scanopy could not send this email. Ask an administrator to check the server \
                 logs for the mail server's response."
            }
            Self::EmailFeedbackRejected => "Not an authentic message for the configured topic",

            // Credentials
            Self::CredentialDefaultExists { .. } => {
                "This credential is identical to the default credential '{name}'. Use that one instead."
            }
            Self::CredentialDefaultDeleteForbidden { .. } => {
                "'{name}' is a default credential and can't be deleted. Unassign it from sites and hosts instead."
            }
            Self::CredentialDefaultSettingsLocked { .. } => {
                "'{name}' is a default credential. Its name, description and tags can change, but its settings can't."
            }
            Self::CredentialLoginRequired => "Choose the SSH login this credential runs over.",
            Self::CredentialLoginInvalid => {
                "The chosen login is not an SSH login of this organization. Choose another."
            }
            Self::CredentialLoginNotUsed => {
                "This type of credential runs over no login. Clear the login and save again."
            }
            Self::CredentialLoginDaemonOsMismatch { .. } => {
                "This credential and its login '{login}' read files on daemons of different OSes. Set both to the same Daemon OS."
            }
            Self::CredentialLoginInUse { .. } => {
                "'{name}' is the login for {users}. Choose another login for them before deleting it."
            }
            Self::CredentialLoginNotAssignable { .. } => {
                "'{name}' is a login and is not assigned to sites or hosts. Assign a credential that uses it instead."
            }

            // Discovery
            Self::DiscoveryHistoricalReadOnly => "Historical discovery cannot be modified via API",
            Self::DiscoverySubnetSiteMismatch { .. } => "Subnet '{subnet}' is on a different site",
            Self::DiscoverySessionNotFound { .. } => "Discovery session '{id}' not found",

            // IP address
            Self::IpAddressOutOfRange { .. } => {
                "IP address '{ip}' is not within subnet '{subnet}' range"
            }

            // Subnet
            Self::SubnetTypeSeededPerSite { .. } => {
                "Every site already has its own {subnet_type} subnet, so a new one can't be created"
            }

            // Daemon
            Self::DaemonSiteMismatch => "Cannot send updates for a different site",
            Self::DaemonIdentityMismatch => "Cannot send updates for a different daemon",
            Self::DaemonStandby => {
                "The daemon is on standby because it has not completed a discovery in over 30 days. Restart the daemon or start a discovery to resume."
            }
            Self::DaemonNotRegistered => {
                "Daemon not found on server. It may have been deleted or the database was reset. Reinstall or reconfigure the daemon."
            }
            Self::DaemonPlanRequired => "Your plan includes one daemon. Choose a plan to add more.",
            Self::DaemonVersionTooOld { .. } => {
                "Daemon version {daemon_version} is no longer supported. This server accepts daemons newer than {last_unsupported_version}. Update the daemon."
            }
            Self::DaemonOsMismatch { .. } => {
                "This daemon was created for {expected} but is running on {actual}. Create a new daemon for this OS, or install this one on the OS it was created for."
            }

            // User
            Self::UserEmailInUse { .. } => "Email '{email}' is already in use",

            // Organizations
            Self::OrganizationHasActiveSubscription => {
                "Cancel your subscription before deleting your organization"
            }

            // Billing
            Self::BillingPaymentRequired => "Payment is required to continue",
            Self::BillingPlanLimitReached { .. } => {
                "You've reached the limit of {limit} {resource} on your current plan"
            }
            Self::BillingSubscriptionRequired => "Active subscription required",
            Self::BillingSetupIncomplete => "Billing setup is incomplete",
            Self::BillingWebhookSignatureInvalid => "The Stripe webhook signature is invalid",
            Self::BillingHostLimitReached { .. } => {
                "You've reached the limit of {limit} hosts on your current plan. Upgrade for unlimited hosts."
            }
            Self::BillingFeatureNotAvailable { .. } => {
                "Your current plan does not include {feature}. Upgrade your plan to access this feature."
            }
            Self::BillingSelfHostedPlanLocked => {
                "This organization is on a self-hosted plan. Manage its license in Settings, or switch to a cloud plan to use the app."
            }
            Self::BillingPlanLapsed => {
                "Your plan has lapsed. Choose a paid plan in Settings to make changes again."
            }
            Self::BillingPlanRequired => "Choose a plan in Settings to use Scanopy.",
            Self::BillingAirGappedKeyStillCurrent => {
                "Your air-gapped key is still valid. You can switch back to an online key once the current license period ends."
            }
            Self::BillingAirGappedPlanChangeBlocked { .. } => {
                "Your air-gapped key covers the plan you have until {date}. You can cancel or move to a cloud plan now, or change license plan from that date."
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
            | Self::ValidationEmailDomainUndeliverable
            | Self::ValidationEmailUndeliverable
            | Self::ValidationEmailDisposable
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
            | Self::AuthDaemonNotProvisioned
            | Self::AuthDemoMode
            | Self::LicenseLocked
            | Self::LicensePlanHasNoKey
            | Self::AuthPasswordLoginDisabled
            | Self::AuthRegistrationDisabled
            | Self::AuthOrgLimitReached
            | Self::AuthEmailVerificationRequired
            | Self::AuthTermsAcceptanceRequired
            | Self::AuthVerificationTokenInvalid
            | Self::AuthOidcNotConfigured
            | Self::SharePasswordRequired
            | Self::SharePasswordIncorrect
            | Self::ShareTokenInvalid
            | Self::InviteAlreadyAccepted
            | Self::InviteEmailMismatch
            | Self::EmailDeliveryFailed
            | Self::EmailFeedbackRejected
            | Self::DiscoveryHistoricalReadOnly
            | Self::DaemonSiteMismatch
            | Self::DaemonIdentityMismatch
            | Self::DaemonStandby
            | Self::DaemonNotRegistered
            | Self::DaemonPlanRequired
            | Self::OrganizationHasActiveSubscription
            | Self::BillingPaymentRequired
            | Self::BillingSubscriptionRequired
            | Self::BillingSetupIncomplete
            | Self::BillingWebhookSignatureInvalid
            | Self::BillingSelfHostedPlanLocked
            | Self::BillingPlanLapsed
            | Self::BillingPlanRequired
            | Self::BillingAirGappedKeyStillCurrent
            | Self::RateLimitExceeded
            | Self::CredentialLoginRequired
            | Self::CredentialLoginInvalid
            | Self::CredentialLoginNotUsed
            | Self::DatabaseError => None,

            // Billing with params
            Self::BillingAirGappedPlanChangeBlocked { date } => Some(json_map! { "date" => date }),

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
            | Self::EntitySiteMismatch { entity } => Some(json_map! { "entity" => entity }),
            Self::EntityDeleteForbidden { entity, reason } => Some(json_map! {
                "entity" => entity,
                "reason" => reason.as_deref().unwrap_or("")
            }),

            // Domain-specific with params
            Self::HostsMergeFailed { reason } => Some(json_map! { "reason" => reason }),
            Self::SitesAccessDenied { site } => Some(json_map! { "site" => site }),
            Self::CredentialDefaultExists { name }
            | Self::CredentialDefaultDeleteForbidden { name }
            | Self::CredentialDefaultSettingsLocked { name }
            | Self::CredentialLoginNotAssignable { name } => Some(json_map! { "name" => name }),
            Self::CredentialLoginDaemonOsMismatch { login } => Some(json_map! { "login" => login }),
            Self::CredentialLoginInUse { name, users } => {
                Some(json_map! { "name" => name, "users" => users })
            }
            Self::DiscoverySubnetSiteMismatch { subnet } => Some(json_map! { "subnet" => subnet }),
            Self::DiscoverySessionNotFound { id } => Some(json_map! {"id" => id}),
            Self::IpAddressOutOfRange { ip, subnet } => {
                Some(json_map! { "ip" => ip, "subnet" => subnet })
            }
            Self::SubnetTypeSeededPerSite { subnet_type } => {
                Some(json_map! { "subnet_type" => subnet_type })
            }
            Self::DaemonVersionTooOld {
                daemon_version,
                last_unsupported_version,
            } => Some(
                json_map! { "daemon_version" => daemon_version, "last_unsupported_version" => last_unsupported_version },
            ),
            Self::DaemonOsMismatch { expected, actual } => {
                Some(json_map! { "expected" => expected, "actual" => actual })
            }
            Self::UserEmailInUse { email } => Some(json_map! { "email" => email }),
            Self::AuthVerificationEmailCooldown { seconds } => {
                Some(json_map! { "seconds" => seconds })
            }
            Self::BillingPlanLimitReached { resource, limit } => {
                Some(json_map! { "resource" => resource, "limit" => limit })
            }
            Self::BillingHostLimitReached { limit } => Some(json_map! { "limit" => limit }),
            Self::LicenseInstanceLimitReached { active, limit } => {
                Some(json_map! { "active" => active, "limit" => limit })
            }
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
