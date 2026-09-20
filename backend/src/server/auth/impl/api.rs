use email_address::EmailAddress;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::organizations::r#impl::base::UseCase;

/// Login request from client
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    /// Email address of the account to sign in to.
    #[schema(value_type = String, format = "email")]
    pub email: EmailAddress,

    // No password-policy validation on login. Login must not reveal the policy
    // (e.g. the minimum length) to unauthenticated callers — that leaks a
    // password-spraying hint. A too-short/wrong password just fails auth
    // generically. The policy is enforced on register / update / reset instead.
    /// The account password.
    #[schema(format = "password", write_only)]
    pub password: String,
}

/// Registration request from client
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    /// Email address for the new account. Must be deliverable.
    #[schema(value_type = String, format = "email")]
    pub email: EmailAddress,

    /// Password for the new account. Minimum 10 characters.
    #[validate(length(min = 10, message = "Password must be at least 10 characters"))]
    #[validate(custom(function = "validate_password_complexity"))]
    #[schema(format = "password", write_only)]
    pub password: String,
    /// Must be `true` — records that the user accepted the terms of service.
    pub terms_accepted: bool,
    /// Whether the user agreed to receive product and marketing email.
    #[serde(default)]
    pub marketing_opt_in: bool,
    /// Honeypot field for bot detection
    #[serde(default, rename = "company_url")]
    pub website: Option<String>,
}

/// Validate password complexity requirements
fn validate_password_complexity(password: &str) -> Result<(), validator::ValidationError> {
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_numeric());

    if !has_uppercase || !has_lowercase || !has_digit {
        let mut err = validator::ValidationError::new("password_complexity");
        err.message = Some("Password must contain uppercase, lowercase, and number".into());
        return Err(err);
    }

    Ok(())
}

/// Check email availability request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckEmailRequest {
    /// Email address to check for an existing account.
    #[schema(value_type = String, format = "email")]
    pub email: EmailAddress,
}

/// Check email availability response.
///
/// An address that is already registered is an answer to this question, not a
/// failure of it: the caller asked whether the address is free and got a
/// truthful no. Returning an error for that case forced every client to either
/// treat a 409 as success or suppress the toast that came with it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckEmailResponse {
    /// Whether the address has no account yet, and so can be registered.
    pub available: bool,
}

/// Session user info (stored in session, not in database)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUser {
    pub user_id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct OidcCallbackParams {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct UpdatePasswordRequest {
    /// Current password — required if the user already has a password set.
    /// Not required for OIDC-only users adding their first password.
    #[schema(format = "password", write_only)]
    pub current_password: Option<String>,
    /// New password to set
    #[validate(length(min = 10, message = "Password must be at least 10 characters"))]
    #[validate(custom(function = "validate_password_complexity"))]
    #[schema(format = "password", write_only)]
    pub new_password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RequestEmailChangeRequest {
    /// Current password — required if the user already has a password set.
    /// Not required for OIDC-only users.
    #[schema(format = "password", write_only)]
    pub current_password: Option<String>,
    /// Address to move the account to. A confirmation link is sent there.
    #[schema(value_type = String, format = "email")]
    pub new_email: EmailAddress,
}

#[derive(Debug, Deserialize)]
pub struct OidcAuthorizeParams {
    pub flow: Option<String>, // "login", "register", or "link"
    pub return_url: Option<String>,
    pub terms_accepted: Option<bool>,
    pub marketing_opt_in: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ForgotPasswordRequest {
    /// Email address to send the password-reset link to.
    #[schema(value_type = String, format = "email")]
    pub email: EmailAddress,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
pub struct ResetPasswordRequest {
    /// Single-use token from the password-reset email.
    #[schema(write_only)]
    pub token: String,
    /// The new password. Minimum 10 characters.
    #[validate(length(min = 10, message = "Password must be at least 10 characters"))]
    #[validate(custom(function = "validate_password_complexity"))]
    #[schema(format = "password", write_only)]
    pub password: String,
}

/// Network configuration for setup
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NetworkSetup {
    /// Name for the network created during setup.
    pub name: String,
}

/// Setup request for pre-registration org/network configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetupRequest {
    /// Name for the organization created during setup.
    pub organization_name: String,
    /// The first network to create alongside the organization.
    pub network: NetworkSetup,
}

/// Response from setup endpoint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetupResponse {
    /// The network this entity belongs to.
    pub network_id: Uuid,
}

/// Request to verify email using token
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyEmailRequest {
    /// Single-use token from the verification email.
    pub token: String,
}

/// Request to resend verification email
#[derive(Debug, Deserialize, ToSchema)]
pub struct ResendVerificationRequest {
    /// Address to resend the verification email to.
    #[schema(value_type = String, format = "email")]
    pub email: EmailAddress,
}

/// Request to save onboarding step
#[derive(Debug, Deserialize, ToSchema)]
pub struct OnboardingStepRequest {
    /// Identifier of the onboarding step the user has reached.
    pub step: String,
    /// Use case selection (homelab, company, msp)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_case: Option<UseCase>,
}

/// Network data in onboarding state response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnboardingNetworkState {
    /// Network ID (if created)
    pub id: Option<Uuid>,
    /// Network name
    pub name: String,
}

/// Response from onboarding state endpoint
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OnboardingStateResponse {
    /// Current onboarding step (if any)
    pub step: Option<String>,
    /// Use case selection (homelab, company, msp)
    pub use_case: Option<UseCase>,
    /// Organization name from pending setup
    pub org_name: Option<String>,
    /// Network from pending setup (with name and ID)
    pub network: Option<OnboardingNetworkState>,
    /// Network ID from pending setup (if any)
    pub network_id: Option<Uuid>,
}

#[cfg(test)]
mod password_policy_tests {
    use super::*;

    // Guards the security property: the password policy (min length + complexity)
    // is enforced on password change and reset, matching registration. These
    // request structs are validated by the handlers before the raw fields reach
    // the service, so a regression that drops the derive would silently allow
    // weak passwords through those flows.

    #[test]
    fn update_password_rejects_weak_and_accepts_strong() {
        let strong = UpdatePasswordRequest {
            current_password: Some("whatever".into()),
            new_password: "Str0ngPassword".into(),
        };
        assert!(strong.validate().is_ok());

        let too_short = UpdatePasswordRequest {
            current_password: None,
            new_password: "Sh0rt".into(),
        };
        assert!(too_short.validate().is_err());

        let no_complexity = UpdatePasswordRequest {
            current_password: None,
            new_password: "alllowercasenodigits".into(),
        };
        assert!(no_complexity.validate().is_err());
    }

    #[test]
    fn reset_password_rejects_weak_and_accepts_strong() {
        let strong = ResetPasswordRequest {
            token: "tok".into(),
            password: "Str0ngPassword".into(),
        };
        assert!(strong.validate().is_ok());

        let too_short = ResetPasswordRequest {
            token: "tok".into(),
            password: "Ab1".into(),
        };
        assert!(too_short.validate().is_err());

        let no_digit = ResetPasswordRequest {
            token: "tok".into(),
            password: "NoDigitsHereAtAll".into(),
        };
        assert!(no_digit.validate().is_err());
    }
}
