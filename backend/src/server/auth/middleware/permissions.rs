//! Composable permission requirements for authorization.
//!
//! This module provides a type-safe, composable system for checking permissions.
//!
//! # Architecture
//!
//! - `PermissionRequirement` trait: Core trait that checks if an entity meets a requirement
//! - `Authorized<P>` extractor: Generic axum extractor that enforces a permission requirement
//! - `Can<T, A>`: requires action `A` on the resource of `T`, per the role table in
//!   `auth/access.rs`, and scopes the handler's sites to that action on that resource
//! - Auth type requirements: `IsDaemon`, `IsUser`, `IsApiKey` - check authentication type
//! - Combinators: `Or<A, B>`, `And<A, B>` - compose requirements
use std::marker::PhantomData;

use crate::server::auth::access::{AccessSet, Action, Resource};
use crate::server::auth::middleware::auth::AuthError;
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::{
    config::AppState,
    shared::{storage::traits::Entity, types::api::ApiError},
    sites::r#impl::Site,
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
// Resource Requirements
// ============================================================================

/// The types a role can grant access to: every entity, by `E::resource()`, plus the markers for
/// resources no entity stands for.
pub trait HasResource: 'static {
    fn resource() -> Resource;
}

impl<T: Entity + 'static> HasResource for T {
    fn resource() -> Resource {
        T::entity_type().resource()
    }
}

/// The organization's subscription.
pub struct Billing;

impl HasResource for Billing {
    fn resource() -> Resource {
        Resource::Billing
    }
}

/// The organization's self-hosted license.
pub struct License;

impl HasResource for License {
    fn resource() -> Resource {
        Resource::License
    }
}

/// Creating a site.
pub struct NewSite;

impl HasResource for NewSite {
    fn resource() -> Resource {
        Resource::NewSite
    }
}

/// An [`Action`] as a type, for [`Can`].
pub trait ActionType: 'static {
    const ACTION: Action;
}

pub struct Read;

impl ActionType for Read {
    const ACTION: Action = Action::Read;
}

pub struct Write;

impl ActionType for Write {
    const ACTION: Action = Action::Write;
}

pub struct Delete;

impl ActionType for Delete {
    const ACTION: Action = Action::Delete;
}

/// Requires holding action `A` on `T`'s resource somewhere. [`Authorized::site_ids`] then returns
/// the sites where `A` on that resource is allowed, which is what the handler's site checks and
/// list filters run against.
///
/// Passes for: User/ApiKey whose role grants `A` on the resource
/// Fails for: everyone else, including Daemon, System and Anonymous
pub struct Can<T, A>(PhantomData<fn() -> (T, A)>);

impl<T: HasResource, A: ActionType> PermissionRequirement for Can<T, A> {
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        if entity.can(T::resource(), A::ACTION) {
            Ok(())
        } else {
            Err(ApiError::permission_denied())
        }
    }

    fn description() -> &'static str {
        "Insufficient permissions"
    }
}

/// A requirement that decides which sites its caller may act on.
pub trait Scoped: PermissionRequirement {
    fn site_ids(entity: &AuthenticatedEntity) -> Vec<Uuid>;
}

impl<T: HasResource, A: ActionType> Scoped for Can<T, A> {
    fn site_ids(entity: &AuthenticatedEntity) -> Vec<Uuid> {
        entity.site_ids_for(T::resource(), A::ACTION)
    }
}

/// A requirement that proves `Can<T, A>`, so a generic handler for `T` can accept it.
pub trait Proves<T, A>: Scoped {}

impl<T: HasResource, A: ActionType> Proves<T, A> for Can<T, A> {}

/// Creating a site is the one write on a site with no site to scope to, so `Can<NewSite, Write>`
/// stands in for `Can<Site, Write>` on `POST /sites`.
impl Proves<Site, Write> for Can<NewSite, Write> {}

// ============================================================================
// Organization Membership
// ============================================================================

/// Requires a user or API key, whatever its role. For readers that span resource types and ask
/// per type through [`AuthenticatedEntity::site_ids_for`], and for routes that decide from the
/// request body which resource they touch.
///
/// Passes for: User, ApiKey
/// Fails for: Daemon, System, Anonymous
pub struct InOrganization;

impl PermissionRequirement for InOrganization {
    fn check(entity: &AuthenticatedEntity) -> Result<(), ApiError> {
        match entity {
            AuthenticatedEntity::User { .. } | AuthenticatedEntity::ApiKey { .. } => Ok(()),
            _ => Err(ApiError::permission_denied()),
        }
    }

    fn description() -> &'static str {
        "User or API key required"
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
/// Use on endpoints that reach third parties or grant lasting access: invites, API keys,
/// shares, user management, emailing the install command. Lifted on a server with no email
/// transport (see [`AuthenticatedEntity::meets_email_verification`]).
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

impl Scoped for IsDaemon {
    fn site_ids(entity: &AuthenticatedEntity) -> Vec<Uuid> {
        entity.daemon_site_id().into_iter().collect()
    }
}

impl<A: Scoped, B: Scoped> Scoped for Or<A, B> {
    fn site_ids(entity: &AuthenticatedEntity) -> Vec<Uuid> {
        let mut site_ids = A::site_ids(entity);
        site_ids.extend(B::site_ids(entity));
        site_ids
    }
}

// The scoped requirement of an `And` is its second parameter: `And<IsUser, Can<T, A>>`.
impl<A: PermissionRequirement, B: Scoped> Scoped for And<A, B> {
    fn site_ids(entity: &AuthenticatedEntity) -> Vec<Uuid> {
        B::site_ids(entity)
    }
}

impl<X: PermissionRequirement, P: Proves<T, A>, T, A> Proves<T, A> for And<X, P> {}

impl<P: Scoped> Scoped for RequireVerified<P> {
    fn site_ids(entity: &AuthenticatedEntity) -> Vec<Uuid> {
        P::site_ids(entity)
    }
}

impl<P: Proves<T, A>, T, A> Proves<T, A> for RequireVerified<P> {}

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

impl<P: Scoped> Authorized<P> {
    /// The sites where the action this requirement checked is allowed.
    pub fn site_ids(&self) -> Vec<Uuid> {
        P::site_ids(&self.entity)
    }
}

impl<P: PermissionRequirement> Authorized<P> {
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

    /// The caller's access, or an error for a caller that holds none.
    pub fn require_access(&self) -> Result<&AccessSet, ApiError> {
        self.entity.access().ok_or_else(ApiError::user_required)
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
        let has_email_service = state.as_ref().services.email_service.is_some();
        if !P::allows_unverified() && !entity.meets_email_verification(has_email_service) {
            return Err(AuthError(ApiError::email_verification_required()));
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
