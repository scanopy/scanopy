//! Request-scoped entity caching utilities.
//!
//! Provides caching for frequently looked-up entities within a single request
//! to avoid duplicate database queries across middleware and handlers.
//!
//! # Usage
//!
//! ```ignore
//! // In middleware or extractors:
//! let org = CachedOrganization::get_or_load(parts, app_state, &org_id).await?;
//!
//! // Subsequent calls with same ID return cached value:
//! let org = CachedOrganization::get_or_load(parts, app_state, &org_id).await?;
//! ```

use crate::server::{
    config::AppState,
    organizations::r#impl::base::Organization,
    shared::{services::traits::CrudService, types::api::ApiError},
    sites::r#impl::Site,
    users::r#impl::base::User,
};
use axum::http::request::Parts;
use uuid::Uuid;

/// Cached organization lookup stored in request extensions.
#[derive(Clone)]
pub struct CachedOrganization(pub Organization);

impl CachedOrganization {
    /// Get organization from cache or load from DB and cache it.
    pub async fn get_or_load(
        parts: &mut Parts,
        app_state: &AppState,
        organization_id: &Uuid,
    ) -> Result<Organization, ApiError> {
        // Check cache first
        if let Some(cached) = parts.extensions.get::<CachedOrganization>()
            && cached.0.id == *organization_id
        {
            return Ok(cached.0.clone());
        }

        // Load from DB
        let organization = app_state
            .services
            .organization_service
            .get_by_id(organization_id)
            .await
            .map_err(|_| ApiError::internal_error("Failed to load organization"))?
            .ok_or_else(|| ApiError::forbidden("Organization not found"))?;

        // Cache for subsequent extractors
        parts
            .extensions
            .insert(CachedOrganization(organization.clone()));

        Ok(organization)
    }
}

/// Cached user lookup stored in request extensions.
#[derive(Clone)]
pub struct CachedUser(pub User);

impl CachedUser {
    /// Get user from cache or load from DB and cache it.
    pub async fn get_or_load(
        parts: &mut Parts,
        app_state: &AppState,
        user_id: &Uuid,
    ) -> Result<User, ApiError> {
        // Check cache first
        if let Some(cached) = parts.extensions.get::<CachedUser>()
            && cached.0.id == *user_id
        {
            return Ok(cached.0.clone());
        }

        // Load from DB
        let user = app_state
            .services
            .user_service
            .get_by_id(user_id)
            .await
            .map_err(|_| ApiError::internal_error("Failed to load user"))?
            .ok_or_else(|| ApiError::forbidden("User not found"))?;

        // Cache for subsequent extractors
        parts.extensions.insert(CachedUser(user.clone()));

        Ok(user)
    }
}

/// Cached site lookup stored in request extensions.
#[derive(Clone)]
pub struct CachedSite(pub Site);

impl CachedSite {
    /// Get site from cache or load from DB and cache it.
    pub async fn get_or_load(
        parts: &mut Parts,
        app_state: &AppState,
        site_id: &Uuid,
    ) -> Result<Site, ApiError> {
        // Check cache first
        if let Some(cached) = parts.extensions.get::<CachedSite>()
            && cached.0.id == *site_id
        {
            return Ok(cached.0.clone());
        }

        // Load from DB
        let site = app_state
            .services
            .site_service
            .get_by_id(site_id)
            .await
            .map_err(|_| ApiError::internal_error("Failed to load site"))?
            .ok_or_else(|| ApiError::forbidden("Site not found"))?;

        // Cache for subsequent extractors
        parts.extensions.insert(CachedSite(site.clone()));

        Ok(site)
    }
}
