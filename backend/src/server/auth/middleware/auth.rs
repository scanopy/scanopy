use std::{fmt::Display, net::IpAddr, str::FromStr};

use cidr::IpCidr;

use crate::server::{
    config::AppState,
    daemon_api_keys::{r#impl::base::DaemonApiKey, service::ResolvedDaemonKey},
    shared::{
        api_key_common::{ApiKeyCommon, ApiKeyType, check_key_validity, hash_api_key},
        events::{
            traits::{AuthScope, Event},
            types::AuthOperation,
        },
        services::traits::CrudService,
        storage::{filter::StorableFilter, traits::Unique},
        types::api::ApiError,
    },
    sites::r#impl::Site,
    users::r#impl::{base::User, permissions::UserOrgPermissions},
};
use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, request::Parts},
    response::{IntoResponse, Response},
};
use axum_client_ip::ClientIp;
use chrono::Utc;
use email_address::EmailAddress;
use serde::Deserialize;
use serde::Serialize;
use tower_sessions::Session;
use uuid::Uuid;

pub struct AuthError(pub ApiError);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}

/// Reject a daemon whose version is below the enforced support floor. Applied to
/// every daemon-authenticated request (see the `ApiKeyType::Daemon` arm), so an
/// unsupported daemon is turned away before it can poll, register, or discover —
/// not only at the handshake.
///
/// Returns the same `DaemonVersionTooOld` error the handshake uses, so the
/// daemon's existing error handling surfaces the prescriptive upgrade message.
///
/// Semantics:
/// - A parseable version below the floor is rejected.
/// - An absent/unparseable version is rejected only once the floor has advanced
///   past the version header's own floor (0.14.10): before that, a missing header
///   legitimately means "old but still allowed" and must not be blocked. This is
///   also what makes the check dormant pre-launch (floor 0.12.0 ⇒ absent is fine).
fn enforce_daemon_version(
    version: Option<&str>,
    now: chrono::DateTime<Utc>,
) -> Result<(), AuthError> {
    use crate::server::daemons::r#impl::version::enforced_floor;
    let floor = enforced_floor(now);
    let too_old = |reported: &str| {
        AuthError(ApiError::daemon_version_too_old(
            reported,
            &floor.to_string(),
        ))
    };
    match version.and_then(|v| semver::Version::parse(v).ok()) {
        Some(v) if v < floor => Err(too_old(&v.to_string())),
        Some(_) => Ok(()),
        None if floor > semver::Version::new(0, 14, 10) => Err(too_old("unknown")),
        None => Ok(()),
    }
}

/// Represents how an entity authenticated - used for audit logging
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// User authenticated via session cookie
    Session,
    /// Authenticated via user API key (scp_u_ prefix)
    UserApiKey,
    /// Authenticated via daemon API key (scp_d_ prefix)
    DaemonApiKey,
    /// External service authentication (e.g., Prometheus)
    ExternalService,
    /// System-level operation (internal)
    System,
    /// No authentication
    Anonymous,
}

impl Display for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthMethod::Session => write!(f, "session"),
            AuthMethod::UserApiKey => write!(f, "user_api_key"),
            AuthMethod::DaemonApiKey => write!(f, "daemon_api_key"),
            AuthMethod::ExternalService => write!(f, "external_service"),
            AuthMethod::System => write!(f, "system"),
            AuthMethod::Anonymous => write!(f, "anonymous"),
        }
    }
}

/// Represents either an authenticated user, daemon, or user API key
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, strum::VariantNames)]
pub enum AuthenticatedEntity {
    User {
        user_id: Uuid,
        organization_id: Uuid,
        permissions: UserOrgPermissions,
        site_ids: Vec<Uuid>,
        email: EmailAddress,
        email_verified: bool,
    },
    Daemon {
        site_id: Uuid,
        api_key_id: Uuid,
        daemon_id: Uuid,
        /// Daemon version from `X-Daemon-Version` header (introduced in v0.14.10)
        version: Option<String>,
    },
    /// User API key authentication - acts on behalf of a user with potentially restricted permissions
    ApiKey {
        api_key_id: Uuid,
        user_id: Uuid,
        organization_id: Uuid,
        permissions: UserOrgPermissions,
        site_ids: Vec<Uuid>,
    },
    /// External service authentication (e.g., Prometheus, Grafana)
    ExternalService {
        /// Service name from X-Service-Name header
        name: String,
    },
    System,
    Anonymous,
}

/// Who acted, in the form sent outside the server with every event: identifiers only, never
/// permissions or site access.
#[derive(Debug, Clone, Serialize)]
pub struct ActorProperties {
    pub auth_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<EmailAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daemon_id: Option<Uuid>,
}

impl From<&AuthenticatedEntity> for ActorProperties {
    fn from(auth: &AuthenticatedEntity) -> Self {
        Self {
            auth_type: auth.entity_name(),
            user_id: auth.user_id(),
            email: auth.email().cloned(),
            organization_id: auth.organization_id(),
            daemon_id: auth.daemon_id(),
        }
    }
}

impl Display for AuthenticatedEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthenticatedEntity::Anonymous => write!(f, "Anonymous"),
            AuthenticatedEntity::System => write!(f, "System"),
            AuthenticatedEntity::Daemon { .. } => write!(f, "Daemon"),
            AuthenticatedEntity::User {
                user_id,
                permissions,
                ..
            } => write!(
                f,
                "User {{ user_id: {}, permissions: {} }}",
                user_id, permissions
            ),
            AuthenticatedEntity::ApiKey {
                api_key_id,
                user_id,
                permissions,
                ..
            } => write!(
                f,
                "ApiKey {{ api_key_id: {}, user_id: {}, permissions: {} }}",
                api_key_id, user_id, permissions
            ),
            AuthenticatedEntity::ExternalService { name } => {
                write!(f, "ExternalService {{ name: {} }}", name)
            }
        }
    }
}

impl AuthenticatedEntity {
    /// Get the user_id if this is a User or ApiKey, otherwise None
    pub fn user_id(&self) -> Option<Uuid> {
        match self {
            AuthenticatedEntity::User { user_id, .. } => Some(*user_id),
            AuthenticatedEntity::ApiKey { user_id, .. } => Some(*user_id),
            _ => None,
        }
    }

    /// Get the organization_id if this is a User or ApiKey, otherwise None
    pub fn organization_id(&self) -> Option<Uuid> {
        match self {
            AuthenticatedEntity::User {
                organization_id, ..
            } => Some(*organization_id),
            AuthenticatedEntity::ApiKey {
                organization_id, ..
            } => Some(*organization_id),
            _ => None,
        }
    }

    /// Get permissions if this is a User or ApiKey, otherwise None
    pub fn permissions(&self) -> Option<UserOrgPermissions> {
        match self {
            AuthenticatedEntity::User { permissions, .. } => Some(*permissions),
            AuthenticatedEntity::ApiKey { permissions, .. } => Some(*permissions),
            _ => None,
        }
    }

    pub fn entity_name(&self) -> String {
        match self {
            AuthenticatedEntity::User { .. } => "user".to_string(),
            AuthenticatedEntity::Daemon { .. } => "daemon".to_string(),
            AuthenticatedEntity::ApiKey { .. } => "api_key".to_string(),
            AuthenticatedEntity::ExternalService { name } => format!("external_service:{}", name),
            AuthenticatedEntity::System => "system".to_string(),
            AuthenticatedEntity::Anonymous => "anonymous".to_string(),
        }
    }

    pub fn entity_id(&self) -> Option<Uuid> {
        match self {
            AuthenticatedEntity::User { user_id, .. } => Some(*user_id),
            AuthenticatedEntity::Daemon { daemon_id, .. } => Some(*daemon_id),
            AuthenticatedEntity::ApiKey { api_key_id, .. } => Some(*api_key_id),
            AuthenticatedEntity::ExternalService { .. } => None,
            AuthenticatedEntity::System => None,
            AuthenticatedEntity::Anonymous => None,
        }
    }

    /// Get site_ids that daemon / user / API key have access to
    pub fn site_ids(&self) -> Vec<Uuid> {
        match self {
            AuthenticatedEntity::Daemon { site_id, .. } => vec![*site_id],
            AuthenticatedEntity::User { site_ids, .. } => site_ids.clone(),
            AuthenticatedEntity::ApiKey { site_ids, .. } => site_ids.clone(),
            AuthenticatedEntity::ExternalService { .. } => vec![], // Global scope, no site restriction
            AuthenticatedEntity::System => vec![],
            AuthenticatedEntity::Anonymous => vec![],
        }
    }

    /// Check if this is a user (session-based authentication)
    pub fn is_user(&self) -> bool {
        matches!(self, AuthenticatedEntity::User { .. })
    }

    /// Check if this is a daemon
    pub fn is_daemon(&self) -> bool {
        matches!(self, AuthenticatedEntity::Daemon { .. })
    }

    /// Check if this is a user API key
    pub fn is_api_key(&self) -> bool {
        matches!(self, AuthenticatedEntity::ApiKey { .. })
    }

    /// Check if this is an external service
    pub fn is_external_service(&self) -> bool {
        matches!(self, AuthenticatedEntity::ExternalService { .. })
    }

    /// Check if this is a user or API key (has user-level permissions)
    pub fn is_user_or_api_key(&self) -> bool {
        matches!(
            self,
            AuthenticatedEntity::User { .. } | AuthenticatedEntity::ApiKey { .. }
        )
    }

    /// Check if this entity has access to the specified site
    pub fn has_site_access(&self, site_id: &Uuid) -> bool {
        self.site_ids().contains(site_id)
    }

    /// Get the email address if this is a User.
    ///
    /// Every other variant returns `None`. `ApiKey` carries `user_id` but no address of its own, so
    /// a caller that needs the key owner's email must look the user up rather than expect it here.
    pub fn email(&self) -> Option<&EmailAddress> {
        match self {
            AuthenticatedEntity::User { email, .. } => Some(email),
            _ => None,
        }
    }

    /// Get daemon_id if this is a Daemon, otherwise None
    pub fn daemon_id(&self) -> Option<Uuid> {
        match self {
            AuthenticatedEntity::Daemon { daemon_id, .. } => Some(*daemon_id),
            _ => None,
        }
    }

    /// Get daemon version if this is a Daemon that sent X-Daemon-Version, otherwise None
    pub fn daemon_version(&self) -> Option<&str> {
        match self {
            AuthenticatedEntity::Daemon { version, .. } => version.as_deref(),
            _ => None,
        }
    }

    /// Check if the entity's email is verified.
    /// Returns true for non-User entities (daemons, API keys, etc.) since
    /// email verification only applies to user sessions.
    pub fn email_verified(&self) -> bool {
        match self {
            AuthenticatedEntity::User { email_verified, .. } => *email_verified,
            _ => true,
        }
    }
}

impl From<User> for AuthenticatedEntity {
    fn from(value: User) -> Self {
        AuthenticatedEntity::User {
            user_id: value.id,
            organization_id: value.base.organization_id,
            permissions: value.base.permissions,
            site_ids: vec![],
            email: value.base.email,
            email_verified: value.base.email_verified,
        }
    }
}

/// Marker to cache failed auth attempts and prevent duplicate event publishing
#[derive(Clone)]
struct AuthAttemptFailed(ApiError);

// Generic authenticated entity extractor - accepts users, daemons, and user API keys
impl<S> FromRequestParts<S> for AuthenticatedEntity
where
    S: Send + Sync + AsRef<AppState>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Check if already extracted (cached in extensions) to avoid duplicate auth
        // This prevents multiple middleware/extractors from triggering repeated DB updates
        if let Some(cached) = parts.extensions.get::<AuthenticatedEntity>() {
            return Ok(cached.clone());
        }

        // Check if auth already failed for this request (prevents duplicate event publishing)
        if let Some(cached_failure) = parts.extensions.get::<AuthAttemptFailed>() {
            return Err(AuthError(cached_failure.0.clone()));
        }

        let result = Self::extract_auth(parts, state).await;

        // Cache result in extensions for subsequent extractors
        match &result {
            Ok(entity) => {
                parts.extensions.insert(entity.clone());
            }
            Err(AuthError(api_error)) => {
                parts
                    .extensions
                    .insert(AuthAttemptFailed(api_error.clone()));
            }
        }

        result
    }
}

impl AuthenticatedEntity {
    /// Internal auth extraction logic - called once and cached
    async fn extract_auth<S>(parts: &mut Parts, state: &S) -> Result<Self, AuthError>
    where
        S: Send + Sync + AsRef<AppState>,
    {
        let app_state = state.as_ref();

        // Extract IP and user agent for failed auth logging
        let ip = ClientIp::from_request_parts(parts, state)
            .await
            .map(|c| c.0)
            .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
        let user_agent = extract_user_agent(&parts.headers);

        // Check for Bearer token in Authorization header
        if let Some(auth_header) = parts.headers.get(axum::http::header::AUTHORIZATION)
            && let Ok(auth_str) = auth_header.to_str()
            && let Some(api_key_raw) = auth_str.strip_prefix("Bearer ")
        {
            // Check for external service token (e.g., Prometheus metrics endpoint)
            // This must come before API key type detection since external tokens don't have prefixes
            if let Some(metrics_token) = &app_state.config.metrics_token
                && !metrics_token.is_empty()
                && api_key_raw == metrics_token
            {
                // External service authentication
                let service_name = parts
                    .headers
                    .get("X-Service-Name")
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_lowercase())
                    .unwrap_or_else(|| "unknown".to_string());

                // Check IP restrictions for this service
                if let Some(allowed_ips) = app_state
                    .config
                    .external_service_allowed_ips
                    .get(&service_name)
                    && !is_ip_allowed(ip, allowed_ips)
                {
                    return Err(AuthError(ApiError::forbidden(&format!(
                        "IP {} not allowed for external service '{}'",
                        ip, service_name
                    ))));
                }

                return Ok(AuthenticatedEntity::ExternalService { name: service_name });
            }

            let hashed_key = hash_api_key(api_key_raw);
            // Extract key prefix for logging (first 8 chars, safe for logging)
            let key_prefix = api_key_raw.get(..8);

            // Detect key type from prefix
            let (key_type, _is_prefixed) = ApiKeyType::from_key(api_key_raw);

            match key_type {
                ApiKeyType::User => {
                    // User API key authentication
                    if let Ok(Some(mut user_api_key)) = app_state
                        .services
                        .user_api_key_service
                        .get_by_key(&hashed_key)
                        .await
                    {
                        let api_key_id = user_api_key.id;
                        let user_id = user_api_key.base.user_id;
                        let organization_id = user_api_key.base.organization_id;
                        let permissions = user_api_key.base.permissions;
                        let service = app_state.services.user_api_key_service.clone();

                        // Check validity using shared trait
                        if let Err(e) = check_key_validity(&user_api_key) {
                            let reason = if user_api_key.is_expired() {
                                "expired"
                            } else {
                                "disabled"
                            };
                            publish_api_key_auth_failed(
                                app_state,
                                ip,
                                user_agent.clone(),
                                key_type,
                                reason,
                                key_prefix,
                            )
                            .await;

                            // Auto-disable expired keys
                            if user_api_key.is_expired() {
                                user_api_key.set_is_enabled(false);
                                tokio::spawn(async move {
                                    let _ = service
                                        .update(&mut user_api_key, AuthenticatedEntity::System)
                                        .await;
                                });
                            }
                            return Err(AuthError(e));
                        }

                        let organization = super::cache::CachedOrganization::get_or_load(
                            parts,
                            app_state,
                            &organization_id,
                        )
                        .await
                        .map_err(AuthError)?;
                        let plan = organization
                            .base
                            .plan
                            .unwrap_or_else(crate::server::billing::plans::get_free_plan);

                        if !plan.features().api_access {
                            return Err(AuthError(ApiError::payment_required(
                                "Your plan does not include api access",
                            )));
                        }

                        // Get site access from junction table
                        let site_ids = app_state
                            .services
                            .user_api_key_service
                            .get_site_ids(&api_key_id)
                            .await
                            .unwrap_or_default();

                        // Update last used asynchronously (don't block auth)
                        user_api_key.set_last_used(Some(Utc::now()));
                        tokio::spawn(async move {
                            let _ = service
                                .update(&mut user_api_key, AuthenticatedEntity::System)
                                .await;
                        });

                        return Ok(AuthenticatedEntity::ApiKey {
                            api_key_id,
                            user_id,
                            organization_id,
                            permissions,
                            site_ids,
                        });
                    }

                    publish_api_key_auth_failed(
                        app_state,
                        ip,
                        user_agent.clone(),
                        key_type,
                        "invalid_key",
                        key_prefix,
                    )
                    .await;
                    return Err(AuthError(ApiError::not_authenticated()));
                }
                ApiKeyType::Daemon => {
                    // Daemon identity comes from the X-Daemon-ID header for legacy
                    // site-shared keys, and from the key itself for 1:1 provisioned
                    // keys. Read the header optionally here (a freshly provisioned daemon
                    // has no id yet on its bootstrap request); requiredness and validation
                    // are decided below, once we know the key's shape. Nil is treated as
                    // absent (the daemon sends nil before it has cached its id).
                    let header_daemon_id = parts
                        .headers
                        .get("X-Daemon-ID")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| Uuid::parse_str(s).ok())
                        .filter(|id| !id.is_nil());

                    // Daemon version header (introduced in v0.14.10) — read once, used
                    // by both the cache fast-path and the DB path below.
                    let daemon_version = parts
                        .headers
                        .get("X-Daemon-Version")
                        .and_then(|h| h.to_str().ok())
                        .map(|s| s.to_string());

                    // Enforce the support floor on EVERY daemon-authenticated request
                    // (not just the handshake): a daemon below the enforced floor is
                    // rejected here, before it can poll, register, or run discovery.
                    // Dormant until the v1.0 launch date is set — the floor is 0.12.0
                    // until then, so this changes nothing for the installed base.
                    enforce_daemon_version(daemon_version.as_deref(), Utc::now())?;

                    let daemon_api_key_service = &app_state.services.daemon_api_key_service;

                    // Fast path: a hot poll loop hits this every ~30s; serve the cached
                    // resolution without touching the DB. Validity is re-evaluated here
                    // (not cached as a verdict); a stale/invalid entry is evicted and we
                    // fall through to the DB path (which also handles auto-disable).
                    if let Some(resolved) =
                        daemon_api_key_service.cached_resolution(&hashed_key).await
                    {
                        let expired = resolved
                            .expires_at
                            .is_some_and(|expires_at| expires_at < Utc::now());
                        if resolved.is_enabled && !expired {
                            match resolve_daemon_identity(
                                resolved.daemon_id,
                                header_daemon_id,
                                app_state,
                                ip,
                                user_agent.clone(),
                                key_type,
                                key_prefix,
                            )
                            .await
                            {
                                Ok(daemon_id) => {
                                    return Ok(AuthenticatedEntity::Daemon {
                                        site_id: resolved.site_id,
                                        api_key_id: resolved.api_key_id,
                                        daemon_id,
                                        version: daemon_version,
                                    });
                                }
                                Err(e) => return Err(e),
                            }
                        }
                        daemon_api_key_service
                            .invalidate_resolution(&hashed_key)
                            .await;
                    }

                    // Check if key exists. Distinguish a genuine lookup ERROR (DB /
                    // tag-hydration failure) from "no such key": a transient error must not
                    // be reported as invalid_key, or a daemon would treat a blip as a bad
                    // key and give up. Only Ok(None) falls through to the not-found path.
                    let api_key_filter =
                        StorableFilter::<DaemonApiKey>::new_from_api_key(hashed_key.clone());
                    let key_lookup = app_state
                        .services
                        .daemon_api_key_service
                        .get_unique(api_key_filter)
                        .await;
                    let found_key = match key_lookup {
                        Ok(Unique::One(key)) => Some(key),
                        Ok(Unique::None) => None,
                        // Two rows for one hashed key means the uniqueness this lookup assumes
                        // is broken. Refusing to authenticate is the only safe reading — the
                        // request presented a credential that identifies no single daemon — and
                        // it is loud, because silently taking one of them would hand a caller
                        // whichever site the database returned first.
                        Ok(Unique::Multiple) => {
                            tracing::error!("Daemon api key hash matched more than one key");
                            return Err(AuthError(ApiError::internal_error(
                                "Daemon API key lookup failed",
                            )));
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Daemon api key lookup failed");
                            return Err(AuthError(ApiError::internal_error(
                                "Daemon API key lookup failed",
                            )));
                        }
                    };
                    if let Some(mut api_key) = found_key {
                        let site_id = api_key.base.site_id;
                        let service = app_state.services.daemon_api_key_service.clone();
                        let api_key_id = api_key.id;
                        // Snapshot the fields the cache needs before `api_key` is moved
                        // into the last-used spawn below.
                        let key_daemon_id = api_key.base.daemon_id;
                        let is_enabled = api_key.base.is_enabled;
                        let expires_at = api_key.base.expires_at;

                        // Check validity using shared trait
                        if let Err(e) = check_key_validity(&api_key) {
                            let reason = if api_key.is_expired() {
                                "expired"
                            } else {
                                "disabled"
                            };
                            publish_api_key_auth_failed(
                                app_state,
                                ip,
                                user_agent.clone(),
                                key_type,
                                reason,
                                key_prefix,
                            )
                            .await;

                            // Auto-disable expired keys
                            if api_key.is_expired() {
                                api_key.set_is_enabled(false);
                                tokio::spawn(async move {
                                    let _ = service
                                        .update(&mut api_key, AuthenticatedEntity::System)
                                        .await;
                                });
                            }
                            return Err(AuthError(e));
                        }

                        // Update last used asynchronously (don't block auth). This goes
                        // through the generic update, which does NOT evict the resolution
                        // cache, so a hot poll loop stays cached between last-used writes.
                        api_key.set_last_used(Some(Utc::now()));
                        tokio::spawn(async move {
                            let _ = service
                                .update(&mut api_key, AuthenticatedEntity::System)
                                .await;
                        });

                        // Populate the resolution cache so the next poll skips the DB.
                        daemon_api_key_service
                            .cache_resolution(
                                &hashed_key,
                                ResolvedDaemonKey {
                                    api_key_id,
                                    site_id,
                                    daemon_id: key_daemon_id,
                                    is_enabled,
                                    expires_at,
                                },
                            )
                            .await;

                        let daemon_id = resolve_daemon_identity(
                            key_daemon_id,
                            header_daemon_id,
                            app_state,
                            ip,
                            user_agent.clone(),
                            key_type,
                            key_prefix,
                        )
                        .await?;

                        return Ok(AuthenticatedEntity::Daemon {
                            site_id,
                            api_key_id,
                            daemon_id,
                            version: daemon_version,
                        });
                    }

                    // Check if this daemon exists to provide a better error message
                    // - If daemon exists: key was rotated/revoked, fail immediately
                    // - If daemon doesn't exist (or no id was presented): onboarding
                    //   scenario, daemon should retry
                    let daemon_exists = match header_daemon_id {
                        Some(id) => app_state
                            .services
                            .daemon_service
                            .get_by_id(&id)
                            .await
                            .map(|d| d.is_some())
                            .unwrap_or(false),
                        None => false,
                    };

                    publish_api_key_auth_failed(
                        app_state,
                        ip,
                        user_agent.clone(),
                        key_type,
                        "invalid_key",
                        key_prefix,
                    )
                    .await;

                    if daemon_exists {
                        return Err(AuthError(ApiError::not_authenticated()));
                    }
                    return Err(AuthError(ApiError::daemon_key_not_yet_active()));
                }
            }
        }

        // Try user authentication (session cookie)
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthError(ApiError::not_authenticated()))?;

        let user_id: Uuid = session
            .get("user_id")
            .await
            .map_err(|_| AuthError(ApiError::not_authenticated()))?
            .ok_or_else(|| AuthError(ApiError::not_authenticated()))?;

        let user = app_state
            .services
            .user_service
            .get_by_id(&user_id)
            .await
            .map_err(|_| AuthError(ApiError::not_authenticated()))?
            .ok_or_else(|| AuthError(ApiError::not_authenticated()))?;

        // Reject sessions issued before the user's current session epoch. Bumping
        // the epoch (on password change/reset) invalidates all sessions that
        // don't carry the new value. Sessions predating the feature have no
        // stored epoch and default to 0, matching a never-bumped user.
        let session_epoch: i64 = session
            .get("session_epoch")
            .await
            .ok()
            .flatten()
            .unwrap_or(0);
        if session_epoch < user.base.session_epoch {
            return Err(AuthError(ApiError::not_authenticated()));
        }

        let site_ids: Vec<Uuid> = if matches!(
            user.base.permissions,
            UserOrgPermissions::Owner | UserOrgPermissions::Admin
        ) {
            let org_filter = StorableFilter::<Site>::new_from_org_id(&user.base.organization_id);

            app_state
                .services
                .site_service
                .get_all(org_filter)
                .await
                .map_err(|_| AuthError(ApiError::internal_error("Failed to load sites")))?
                .iter()
                .map(|n| n.id)
                .collect()
        } else {
            // Load site_ids from junction table for non-admin users
            app_state
                .services
                .user_service
                .get_site_ids(&user.id)
                .await
                .map_err(|_| AuthError(ApiError::internal_error("Failed to load user sites")))?
        };

        Ok(AuthenticatedEntity::User {
            user_id: user.id,
            organization_id: user.base.organization_id,
            permissions: user.base.permissions,
            site_ids,
            email: user.base.email,
            email_verified: user.base.email_verified,
        })
    }
}

/// Helper to extract user agent from headers
fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

/// Check if an IP address is allowed based on a list of IP/CIDR strings.
///
/// Returns true if:
/// - The allowed list is empty (no restriction)
/// - The IP matches any entry in the list (exact IP or CIDR range)
///
/// Invalid CIDR entries in the list are logged and skipped.
fn is_ip_allowed(ip: IpAddr, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true; // No restriction = allow all
    }

    for entry in allowed {
        // Try to parse as CIDR first
        if let Ok(cidr) = IpCidr::from_str(entry) {
            if cidr.contains(&ip) {
                return true;
            }
        } else if let Ok(exact_ip) = IpAddr::from_str(entry) {
            // Try as exact IP address
            if ip == exact_ip {
                return true;
            }
        } else {
            // Log invalid entry but don't fail
            tracing::warn!(
                "Invalid IP/CIDR entry in external service allowed IPs: {}",
                entry
            );
        }
    }

    false
}

/// Publish a failed API key authentication event
/// The outcome of resolving a daemon's identity from its key shape and the
/// presented header. Pure and side-effect free so the coexistence matrix is
/// unit-testable; [`resolve_daemon_identity`] maps it to auth events/errors.
#[derive(Debug, PartialEq, Eq)]
enum DaemonIdentityDecision {
    /// Identity resolved to this daemon.
    Resolved(Uuid),
    /// 1:1 key, but the header names a different daemon — the key is being
    /// reused from another daemon. Reject.
    Mismatch,
    /// Legacy site-shared key with no header — identity is unknowable. Reject.
    MissingHeader,
}

/// See the coexistence spine: a 1:1 provisioned key (`key_daemon_id = Some`) is
/// authoritative and a present header must match it; a legacy site-shared key
/// (`None`) takes identity from the required header.
fn decide_daemon_identity(
    key_daemon_id: Option<Uuid>,
    header_daemon_id: Option<Uuid>,
) -> DaemonIdentityDecision {
    match key_daemon_id {
        Some(bound_daemon_id) => match header_daemon_id {
            Some(hdr) if hdr != bound_daemon_id => DaemonIdentityDecision::Mismatch,
            _ => DaemonIdentityDecision::Resolved(bound_daemon_id),
        },
        None => match header_daemon_id {
            Some(hdr) => DaemonIdentityDecision::Resolved(hdr),
            None => DaemonIdentityDecision::MissingHeader,
        },
    }
}

/// Resolve daemon identity, emitting an auth-failed event on a key-reuse mismatch.
/// Shared by the cache fast-path and the DB path.
#[allow(clippy::too_many_arguments)]
async fn resolve_daemon_identity(
    key_daemon_id: Option<Uuid>,
    header_daemon_id: Option<Uuid>,
    app_state: &AppState,
    ip: IpAddr,
    user_agent: Option<String>,
    key_type: ApiKeyType,
    key_prefix: Option<&str>,
) -> Result<Uuid, AuthError> {
    match decide_daemon_identity(key_daemon_id, header_daemon_id) {
        DaemonIdentityDecision::Resolved(id) => Ok(id),
        DaemonIdentityDecision::Mismatch => {
            publish_api_key_auth_failed(
                app_state,
                ip,
                user_agent,
                key_type,
                "daemon_mismatch",
                key_prefix,
            )
            .await;
            Err(AuthError(ApiError::not_authenticated()))
        }
        DaemonIdentityDecision::MissingHeader => Err(AuthError(ApiError::daemon_required())),
    }
}

async fn publish_api_key_auth_failed(
    app_state: &AppState,
    ip: IpAddr,
    user_agent: Option<String>,
    key_type: ApiKeyType,
    reason: &str,
    key_prefix: Option<&str>,
) {
    let event = Event::new(
        AuthScope {
            user_id: None,
            organization_id: None,
            ip_address: ip,
            user_agent,
        },
        AuthOperation::ApiKeyAuthFailed {
            key_type,
            reason: reason.to_string(),
            key_prefix: key_prefix.map(|s| s.to_string()).unwrap_or_default(),
        },
        AuthenticatedEntity::Anonymous,
    );

    // Fire and forget - don't block auth on event publishing
    let event_bus = app_state.services.event_bus.clone();
    tokio::spawn(async move {
        if let Err(e) = event_bus.publish(event).await {
            tracing::warn!(error = %e, "Failed to publish API key auth failed event");
        }
    });
}

/// Extractor that only accepts user API key authentication (rejects users and daemons)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthenticatedApiKey {
    pub api_key_id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub permissions: UserOrgPermissions,
    pub site_ids: Vec<Uuid>,
}

impl From<AuthenticatedApiKey> for AuthenticatedEntity {
    fn from(value: AuthenticatedApiKey) -> Self {
        AuthenticatedEntity::ApiKey {
            api_key_id: value.api_key_id,
            user_id: value.user_id,
            organization_id: value.organization_id,
            permissions: value.permissions,
            site_ids: value.site_ids,
        }
    }
}

impl<S> FromRequestParts<S> for AuthenticatedApiKey
where
    S: Send + Sync + AsRef<AppState>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let entity = AuthenticatedEntity::from_request_parts(parts, state).await?;

        match entity {
            AuthenticatedEntity::ApiKey {
                api_key_id,
                user_id,
                organization_id,
                permissions,
                site_ids,
            } => Ok(AuthenticatedApiKey {
                api_key_id,
                user_id,
                organization_id,
                permissions,
                site_ids,
            }),
            _ => Err(AuthError(ApiError::api_key_required())),
        }
    }
}

#[cfg(test)]
mod daemon_identity_tests {
    use super::{DaemonIdentityDecision, decide_daemon_identity};
    use uuid::Uuid;

    // Coexistence matrix: identity resolution must hold across old/new daemons
    // (which do or don't send a header) and old/new keys (legacy shared vs 1:1).

    #[test]
    fn one_to_one_key_matching_header_resolves_to_the_bound_daemon() {
        let bound = Uuid::new_v4();
        assert_eq!(
            decide_daemon_identity(Some(bound), Some(bound)),
            DaemonIdentityDecision::Resolved(bound)
        );
    }

    #[test]
    fn one_to_one_key_without_header_resolves_from_the_key_for_bootstrap() {
        // A freshly provisioned daemon has no cached id yet on its first request.
        let bound = Uuid::new_v4();
        assert_eq!(
            decide_daemon_identity(Some(bound), None),
            DaemonIdentityDecision::Resolved(bound)
        );
    }

    #[test]
    fn one_to_one_key_with_mismatched_header_is_rejected_as_key_reuse() {
        // Daemon B pasted daemon A's key: B still sends its own id.
        let bound = Uuid::new_v4();
        let other = Uuid::new_v4();
        assert_eq!(
            decide_daemon_identity(Some(bound), Some(other)),
            DaemonIdentityDecision::Mismatch
        );
    }

    #[test]
    fn legacy_key_takes_identity_from_the_required_header() {
        let hdr = Uuid::new_v4();
        assert_eq!(
            decide_daemon_identity(None, Some(hdr)),
            DaemonIdentityDecision::Resolved(hdr)
        );
    }

    #[test]
    fn legacy_key_without_a_header_is_rejected() {
        assert_eq!(
            decide_daemon_identity(None, None),
            DaemonIdentityDecision::MissingHeader
        );
    }
}
