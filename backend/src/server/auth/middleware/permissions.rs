//! Composable permission requirements for authorization.
//!
//! This module provides a type-safe, composable system for checking permissions.
//!
//! # Architecture
//!
//! - `PermissionRequirement` trait: Core trait that checks if an entity meets a requirement
//! - `Authorized<P>` extractor: Generic axum extractor that enforces a permission requirement
//! - Permission levels: `Viewer`, `Member`, `Admin`, `Owner` - check User/ApiKey permission levels
//! - Auth type requirements: `IsDaemon`, `IsUser`, `IsApiKey` - check authentication type
//! - Combinators: `Or<A, B>` - compose requirements
use std::marker::PhantomData;

use crate::server::auth::middleware::auth::AuthError;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::{
    config::AppState, shared::types::api::ApiError, users::r#impl::permissions::UserOrgPermissions,
};
use axum::{extract::FromRequestParts, http::request::Parts};
use uuid::Uuid;

// ============================================================================
// Core Trait
// ============================================================================

/// A permission requirement that can be checked against an authenticated entity.
///
/// Implement this trait to create custom permission checks. Requirements can be
/// composed using `Or<A, B>` and `And<A, B>` combinators.
pub trait PermissionRequirement: Send + Sync + 'static {
    /// Check if the entity meets this permission requirement.
    ///
    /// Returns `Ok(())` if the requirement is met, or an appropriate `ApiError` if not.
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError>;

    /// Human-readable description of this requirement for error messages.
    fn description() -> &'static str;

    /// Whether this requirement allows unverified email users.
    /// Defaults to `true` — most endpoints allow unverified users (soft verification).
    /// Use `RequireVerified<P>` to restrict specific sensitive endpoints.
    fn allows_unverified() -> bool {
        true
    }
}

// ============================================================================
// Permission Level Requirements (for User/ApiKey only)
// ============================================================================

/// Requires Viewer or higher permission level.
///
/// Passes for: User/ApiKey with Viewer, Member, Admin, or Owner
/// Fails for: Daemon, System, Anonymous
pub struct Viewer;

impl PermissionRequirement for Viewer {
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        match entity {
            AuthenticatedEntity::User { permissions, .. }
            | AuthenticatedEntity::ApiKey { permissions, .. } => {
                if *permissions >= UserOrgPermissions::Viewer {
                    Ok(())
                } else {
                    Err(ApiError::permission_denied())
                }
            }
            _ => Err(ApiError::permission_denied()),
        }
    }

    fn description() -> &'static str {
        "Viewer permission required"
    }
}

/// Requires Member or higher permission level.
///
/// Passes for: User/ApiKey with Member, Admin, or Owner
/// Fails for: User/ApiKey with Viewer, Daemon, System, Anonymous
pub struct Member;

impl PermissionRequirement for Member {
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        match entity {
            AuthenticatedEntity::User { permissions, .. }
            | AuthenticatedEntity::ApiKey { permissions, .. } => {
                if *permissions >= UserOrgPermissions::Member {
                    Ok(())
                } else {
                    Err(ApiError::permission_denied())
                }
            }
            _ => Err(ApiError::permission_denied()),
        }
    }

    fn description() -> &'static str {
        "Member permission required"
    }
}

/// Requires Admin or higher permission level.
///
/// Passes for: User/ApiKey with Admin or Owner
/// Fails for: User/ApiKey with Member or Viewer, Daemon, System, Anonymous
pub struct Admin;

impl PermissionRequirement for Admin {
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        match entity {
            AuthenticatedEntity::User { permissions, .. }
            | AuthenticatedEntity::ApiKey { permissions, .. } => {
                if *permissions >= UserOrgPermissions::Admin {
                    Ok(())
                } else {
                    Err(ApiError::permission_denied())
                }
            }
            _ => Err(ApiError::permission_denied()),
        }
    }

    fn description() -> &'static str {
        "Admin permission required"
    }
}

/// Requires Owner permission level.
///
/// Passes for: User/ApiKey with Owner
/// Fails for: User/ApiKey with Admin, Member, or Viewer, Daemon, System, Anonymous
pub struct Owner;

impl PermissionRequirement for Owner {
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        match entity {
            AuthenticatedEntity::User { permissions, .. }
            | AuthenticatedEntity::ApiKey { permissions, .. } => {
                if *permissions >= UserOrgPermissions::Owner {
                    Ok(())
                } else {
                    Err(ApiError::permission_denied())
                }
            }
            _ => Err(ApiError::permission_denied()),
        }
    }

    fn description() -> &'static str {
        "Owner permission required"
    }
}

// ============================================================================
// Auth Type Requirements
// ============================================================================

/// Requires authentication via user session.
///
/// Passes for: User (session-based)
/// Fails for: ApiKey, Daemon, System, Anonymous
pub struct IsUser;

impl PermissionRequirement for IsUser {
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        match entity {
            AuthenticatedEntity::User { .. } => Ok(()),
            _ => Err(ApiError::user_required()),
        }
    }

    fn description() -> &'static str {
        "User session required"
    }
}

/// Requires authentication via API key.
///
/// Passes for: ApiKey
/// Fails for: User, Daemon, System, Anonymous
pub struct IsApiKey;

impl PermissionRequirement for IsApiKey {
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        match entity {
            AuthenticatedEntity::ApiKey { .. } => Ok(()),
            _ => Err(ApiError::api_key_required()),
        }
    }

    fn description() -> &'static str {
        "API key required"
    }
}

/// Requires authentication via daemon.
///
/// Passes for: Daemon
/// Fails for: User, ApiKey, System, Anonymous
pub struct IsDaemon;

impl PermissionRequirement for IsDaemon {
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        match entity {
            AuthenticatedEntity::Daemon { .. } => Ok(()),
            _ => Err(ApiError::daemon_required()),
        }
    }

    fn description() -> &'static str {
        "Daemon authentication required"
    }
}

/// Requires system-level authentication.
///
/// Passes for: System
/// Fails for: User, ApiKey, Daemon, Anonymous
pub struct IsSystem;

impl PermissionRequirement for IsSystem {
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        match entity {
            AuthenticatedEntity::System => Ok(()),
            _ => Err(ApiError::permission_denied()),
        }
    }

    fn description() -> &'static str {
        "System authentication required"
    }
}

// ============================================================================
// External Service Requirements
// ============================================================================

/// Marker trait for external service types.
///
/// Implement this trait to create service-specific permission requirements.
pub trait ExternalServiceType: Send + Sync + 'static {
    /// Returns the expected service name, or None if any service is allowed.
    fn required_name() -> Option<&'static str>;
}

/// Marker for any external service (no name restriction).
pub struct AnyService;

impl ExternalServiceType for AnyService {
    fn required_name() -> Option<&'static str> {
        None
    }
}

/// Marker for Prometheus service.
pub struct Prometheus;

impl ExternalServiceType for Prometheus {
    fn required_name() -> Option<&'static str> {
        Some("prometheus")
    }
}

/// Marker for Grafana service.
pub struct Grafana;

impl ExternalServiceType for Grafana {
    fn required_name() -> Option<&'static str> {
        Some("grafana")
    }
}

/// Requires external service authentication, optionally restricted to a specific service.
///
/// Usage:
/// - `Authorized<IsExternalService>` - any external service
/// - `Authorized<IsExternalService<Prometheus>>` - only prometheus
/// - `Authorized<IsExternalService<Grafana>>` - only grafana
///
/// Passes for: ExternalService (with matching name if T specifies one)
/// Fails for: User, ApiKey, Daemon, System, Anonymous
pub struct IsExternalService<T: ExternalServiceType = AnyService>(PhantomData<T>);

impl<T: ExternalServiceType> PermissionRequirement for IsExternalService<T> {
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        match entity {
            AuthenticatedEntity::ExternalService { name } => {
                if let Some(required_name) = T::required_name()
                    && name.to_lowercase() != required_name
                {
                    return Err(ApiError::permission_denied());
                }
                Ok(())
            }
            _ => Err(ApiError::permission_denied()),
        }
    }

    fn description() -> &'static str {
        "External service authentication required"
    }
}

// ============================================================================
// Combinators
// ============================================================================

/// Requires either A or B to pass.
///
/// This allows composing requirements like `Or<Member, IsDaemon>` which passes
/// if the entity is either a Member-level user/API key OR a daemon.
pub struct Or<A, B>(PhantomData<(A, B)>);

impl<A, B> PermissionRequirement for Or<A, B>
where
    A: PermissionRequirement,
    B: PermissionRequirement,
{
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        if A::check(entity).is_ok() || B::check(entity).is_ok() {
            Ok(())
        } else {
            Err(ApiError::permission_denied())
        }
    }

    fn description() -> &'static str {
        "Insufficient permissions"
    }

    fn allows_unverified() -> bool {
        A::allows_unverified() || B::allows_unverified()
    }
}

/// Requires both A and B to pass.
///
/// This allows composing requirements like `And<Member, HasFeature<"advanced">>`.
pub struct And<A, B>(PhantomData<(A, B)>);

impl<A, B> PermissionRequirement for And<A, B>
where
    A: PermissionRequirement,
    B: PermissionRequirement,
{
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        A::check(entity)?;
        B::check(entity)?;
        Ok(())
    }

    fn description() -> &'static str {
        "Insufficient permissions"
    }

    fn allows_unverified() -> bool {
        A::allows_unverified() && B::allows_unverified()
    }
}

// ============================================================================
// Verified Email Wrapper
// ============================================================================

/// Wraps a permission requirement to require verified email.
/// Use on sensitive endpoints: invites, API keys, billing portal, shares.
pub struct RequireVerified<P>(PhantomData<P>);

impl<P: PermissionRequirement> PermissionRequirement for RequireVerified<P> {
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        P::check(entity)
    }

    fn description() -> &'static str {
        P::description()
    }

    fn allows_unverified() -> bool {
        false
    }
}

// ============================================================================
// Satisfies Trait (Upgrade Guard)
// ============================================================================

/// Marker: P satisfies Q (P is at least as strict as Q).
/// Prevents `into_permission` from upgrading permissions.
pub trait Satisfies<Q: PermissionRequirement>: PermissionRequirement {}

// Level hierarchy: Owner > Admin > Member > Viewer
impl Satisfies<Owner> for Owner {}
impl Satisfies<Admin> for Owner {}
impl Satisfies<Member> for Owner {}
impl Satisfies<Viewer> for Owner {}

impl Satisfies<Admin> for Admin {}
impl Satisfies<Member> for Admin {}
impl Satisfies<Viewer> for Admin {}

impl Satisfies<Member> for Member {}
impl Satisfies<Viewer> for Member {}

impl Satisfies<Viewer> for Viewer {}

// RequireVerified<P> satisfies whatever P satisfies
impl<P, Q> Satisfies<Q> for RequireVerified<P>
where
    P: PermissionRequirement + Satisfies<Q>,
    Q: PermissionRequirement,
{
}

// And<A, B> satisfies Q if B satisfies Q.
// Convention: And<AuthType, Level> — the level (B) determines hierarchy.
impl<A, B, Q> Satisfies<Q> for And<A, B>
where
    A: PermissionRequirement,
    B: PermissionRequirement + Satisfies<Q>,
    Q: PermissionRequirement,
{
}

// ============================================================================
// The Authorized Extractor
// ============================================================================

/// An axum extractor that enforces a permission requirement.
///
/// This is the primary way to protect endpoints. It extracts the authenticated
/// entity, checks the permission requirement, and provides access to the entity.
pub struct Authorized<P: PermissionRequirement> {
    /// The authenticated entity that passed the permission check.
    pub entity: AuthenticatedEntity,
    _marker: PhantomData<P>,
}

impl<P: PermissionRequirement> Authorized<P> {
    /// Get the site IDs this entity has access to.
    pub fn site_ids(&self) -> Vec<Uuid> {
        self.entity.site_ids()
    }

    /// Get the organization ID, if applicable.
    pub fn organization_id(&self) -> Option<Uuid> {
        self.entity.organization_id()
    }

    /// Get the user ID, if applicable.
    pub fn user_id(&self) -> Option<Uuid> {
        self.entity.user_id()
    }

    /// Consume and return the underlying entity.
    pub fn into_entity(self) -> AuthenticatedEntity {
        self.entity
    }

    /// Get the daemon ID, if this is a daemon.
    pub fn daemon_id(&self) -> Option<Uuid> {
        self.entity.daemon_id()
    }

    /// Get the user ID, or return an error if not available.
    pub fn require_user_id(&self) -> Result<Uuid, ApiError> {
        self.entity
            .user_id()
            .ok_or_else(|| ApiError::internal_error("User ID required"))
    }

    /// Get the organization ID, or return an error if not available.
    pub fn require_organization_id(&self) -> Result<Uuid, ApiError> {
        self.entity
            .organization_id()
            .ok_or_else(|| ApiError::internal_error("Organization ID required"))
    }

    /// Get the permissions, or return an error if not available.
    pub fn require_permissions(&self) -> Result<UserOrgPermissions, ApiError> {
        self.entity
            .permissions()
            .ok_or_else(|| ApiError::internal_error("Permissions required"))
    }

    /// Convert to a different permission level.
    /// Use this when a handler requires a lower permission level than what was checked.
    /// For example, Admin -> Member, since Admin >= Member.
    /// The `Satisfies<Q>` bound prevents upgrading permissions (e.g. Member -> Admin).
    pub fn into_permission<Q: PermissionRequirement>(self) -> Authorized<Q>
    where
        P: Satisfies<Q>,
    {
        Authorized {
            entity: self.entity,
            _marker: PhantomData,
        }
    }
}

impl<S, P> FromRequestParts<S> for Authorized<P>
where
    S: Send + Sync + AsRef<AppState>,
    P: PermissionRequirement,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract the authenticated entity
        let entity = AuthenticatedEntity::from_request_parts(parts, state).await?;

        // Check the permission requirement
        P::check(&entity).map_err(AuthError)?;

        // Check email verification unless the endpoint explicitly allows unverified users
        if !P::allows_unverified() && !entity.email_verified() {
            return Err(AuthError(ApiError::coded(
                axum::http::StatusCode::FORBIDDEN,
                crate::server::shared::types::error_codes::ErrorCode::AuthEmailVerificationRequired,
            )));
        }

        Ok(Authorized {
            entity,
            _marker: PhantomData,
        })
    }
}

// Implement conversion to AuthenticatedEntity for easy use in services
impl<P: PermissionRequirement> From<Authorized<P>> for AuthenticatedEntity {
    fn from(auth: Authorized<P>) -> Self {
        auth.entity
    }
}
