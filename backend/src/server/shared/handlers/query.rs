use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use uuid::Uuid;

use crate::server::shared::storage::{filter::StorableFilter, traits::Storable};

// ============================================================================
// Pagination Parameters
// ============================================================================

/// Pagination parameters that can be composed into filter queries.
///
/// Default behavior:
/// - `limit`: 50 (returns up to 50 results)
/// - `offset`: 0 (starts from the beginning)
/// - `limit=0`: No limit (returns all results)
/// - `limit` values above 1000 are capped to 1000
#[derive(Deserialize, Default, Debug, Clone, IntoParams, utoipa::ToSchema)]
pub struct PaginationParams {
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl PaginationParams {
    /// Default limit when not specified
    pub const DEFAULT_LIMIT: u32 = 50;
    /// Maximum allowed limit (0 means unlimited)
    pub const MAX_LIMIT: u32 = 1000;

    /// Get the effective limit, applying defaults and caps.
    /// Returns None if unlimited (limit=0).
    pub fn effective_limit(&self) -> Option<u32> {
        match self.limit {
            Some(0) => None, // Unlimited
            Some(n) => Some(n.min(Self::MAX_LIMIT)),
            None => Some(Self::DEFAULT_LIMIT),
        }
    }

    /// Get the effective offset, defaulting to 0.
    pub fn effective_offset(&self) -> u32 {
        self.offset.unwrap_or(0)
    }

    /// Apply pagination to an StorableFilter.
    pub fn apply_to_filter<T: Storable>(&self, filter: StorableFilter<T>) -> StorableFilter<T> {
        let filter = if let Some(limit) = self.effective_limit() {
            filter.limit(limit)
        } else {
            filter
        };
        filter.offset(self.effective_offset())
    }
}

// ============================================================================
// Order Direction
// ============================================================================

/// Direction for ORDER BY clauses.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum OrderDirection {
    #[default]
    Asc,
    Desc,
}

impl OrderDirection {
    /// Convert to SQL ORDER BY direction keyword.
    pub fn to_sql(&self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}

// ============================================================================
// Filter Query Extractor Trait
// ============================================================================

/// Trait for query structs that filter entities by site or organization.
pub trait FilterQueryExtractor: DeserializeOwned + Send + Sync + Default {
    /// Apply query parameters to the filter, respecting user's access permissions.
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_site_ids: &[Uuid],
        user_organization_id: Uuid,
    ) -> StorableFilter<T>;

    /// Get pagination parameters from the query.
    fn pagination(&self) -> PaginationParams;

    /// As-of timestamp for SCD2 reads. When `Some(t)`, the read path uses
    /// `as_of(t)` (snapshot-pinned state); when `None`, it uses `live()`.
    /// Defaults to `None`; query types that expose an `at` param override this.
    fn at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        None
    }
}

// ============================================================================
// Standard filter query types for CrudHandlers
// ============================================================================

/// Filter query for entities keyed by site_id.
/// Allows filtering to a specific site the user has access to.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct SiteFilterQuery {
    /// Filter by site ID
    pub site_id: Option<Uuid>,
    /// Filter by specific entity IDs (for selective loading)
    pub ids: Option<Vec<Uuid>>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl FilterQueryExtractor for SiteFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        // Apply IDs filter first if provided
        let filter = match &self.ids {
            Some(ids) if !ids.is_empty() => filter.entity_ids(ids),
            _ => filter,
        };
        // Then apply site filter
        match self.site_id {
            Some(id) if user_site_ids.contains(&id) => filter.site_ids(&[id]),
            Some(_) => filter.site_ids(&[]), // User doesn't have access - return empty
            None => filter.site_ids(user_site_ids),
        }
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

/// Empty filter query for entities that are scoped to org (or are the org itself) and don't support further filtering by query param
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct NoFilterQuery {
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl FilterQueryExtractor for NoFilterQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        _user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        // Don't apply additional filters (site_id / org_id permissioning is taken care of in handler)
        filter
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

/// Query for filtering by group_id (used by GroupBinding).
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct GroupIdQuery {
    /// Filter by group ID
    pub group_id: Uuid,
    /// Filter by site ID
    pub site_id: Uuid,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

// ============================================================================
// Combined query types for child entities with site filtering
// ============================================================================

/// Query for filtering ports by host_id and/or site_id.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct HostChildQuery {
    /// Filter by host ID
    pub host_id: Option<Uuid>,
    /// Filter by site ID
    pub site_id: Option<Uuid>,
    /// Filter by specific entity IDs (for selective loading)
    pub ids: Option<Vec<Uuid>>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
    /// As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
    /// instant (snapshot view) instead of live state.
    pub at: Option<chrono::DateTime<chrono::Utc>>,
}

impl FilterQueryExtractor for HostChildQuery {
    fn at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.at
    }

    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        // Apply IDs filter first if provided
        let filter = match &self.ids {
            Some(ids) if !ids.is_empty() => filter.entity_ids(ids),
            _ => filter,
        };
        // Then apply site filter
        let filter = match self.site_id {
            Some(id) if user_site_ids.contains(&id) => filter.site_ids(&[id]),
            Some(_) => filter.site_ids(&[]),
            None => filter.site_ids(user_site_ids),
        };
        // Then apply host filter
        match self.host_id {
            Some(id) => filter.host_id(&id),
            None => filter,
        }
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

/// Query for filtering bindings by service_id and/or site_id.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct BindingQuery {
    /// Filter by service ID
    pub service_id: Option<Uuid>,
    /// Filter by site ID
    pub site_id: Option<Uuid>,
    /// Filter by port ID
    pub port_id: Option<Uuid>,
    /// Filter by interface ID
    pub ip_address_id: Option<Uuid>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
    /// As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
    /// instant (snapshot view) instead of live state.
    pub at: Option<chrono::DateTime<chrono::Utc>>,
}

impl FilterQueryExtractor for BindingQuery {
    fn at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.at
    }

    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        let mut filter = match self.site_id {
            Some(id) if user_site_ids.contains(&id) => filter.site_ids(&[id]),
            Some(_) => filter.site_ids(&[]),
            None => filter.site_ids(user_site_ids),
        };
        filter = match self.service_id {
            Some(id) => filter.service_id(&id),
            None => filter,
        };
        filter = match self.port_id {
            Some(id) => filter.uuid_column("port_id", &id),
            None => filter,
        };
        filter = match self.ip_address_id {
            Some(id) => filter.uuid_column("ip_address_id", &id),
            None => filter,
        };

        filter
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

/// Query for filtering ip_addresses by host_id, subnet_id, and/or site_id.
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct IPAddressQuery {
    /// Filter by host ID
    pub host_id: Option<Uuid>,
    /// Filter by subnet ID
    pub subnet_id: Option<Uuid>,
    /// Filter by site ID
    pub site_id: Option<Uuid>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
    /// As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
    /// instant (snapshot view) instead of live state.
    pub at: Option<chrono::DateTime<chrono::Utc>>,
}

impl FilterQueryExtractor for IPAddressQuery {
    fn at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.at
    }

    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        let mut filter = match self.site_id {
            Some(id) if user_site_ids.contains(&id) => filter.site_ids(&[id]),
            Some(_) => filter.site_ids(&[]),
            None => filter.site_ids(user_site_ids),
        };
        filter = match self.subnet_id {
            Some(id) => filter.subnet_id(&id),
            None => filter,
        };
        filter = match self.host_id {
            Some(id) => filter.host_id(&id),
            None => filter,
        };

        filter
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}

/// Query for filtering shares by site_id or topology_id
#[derive(Deserialize, Default, Debug, Clone, IntoParams)]
pub struct SharesQuery {
    /// Filter by site ID
    pub site_id: Option<Uuid>,
    /// Filter by topology ID
    pub topology_id: Option<Uuid>,
    /// Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
    #[param(minimum = 0, maximum = 1000)]
    pub limit: Option<u32>,
    /// Number of results to skip. Default: 0.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl FilterQueryExtractor for SharesQuery {
    fn apply_to_filter<T: Storable>(
        &self,
        filter: StorableFilter<T>,
        user_site_ids: &[Uuid],
        _user_organization_id: Uuid,
    ) -> StorableFilter<T> {
        let mut filter = match self.site_id {
            Some(id) if user_site_ids.contains(&id) => filter.site_ids(&[id]),
            Some(_) => filter.site_ids(&[]),
            None => filter.site_ids(user_site_ids),
        };
        filter = match self.topology_id {
            Some(id) => filter.uuid_column("topology_id", &id),
            None => filter,
        };

        filter
    }

    fn pagination(&self) -> PaginationParams {
        PaginationParams {
            limit: self.limit,
            offset: self.offset,
        }
    }
}
