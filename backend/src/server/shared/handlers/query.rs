use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use uuid::Uuid;

use crate::server::shared::types::api::ApiError;

use crate::server::shared::storage::{
    filter::StorableFilter,
    traits::{Entity, Storable},
};

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

/// Parse `tag_sets` query values: each a comma-separated list of tag ids. Blank entries are
/// skipped.
pub fn parse_tag_sets(sets: Option<&[String]>) -> Result<Vec<Vec<Uuid>>, ApiError> {
    sets.unwrap_or_default()
        .iter()
        .filter(|set| !set.trim().is_empty())
        .map(|set| {
            set.split(',')
                .map(|id| {
                    Uuid::parse_str(id.trim()).map_err(|_| {
                        ApiError::bad_request(&format!("\"{id}\" in tag_sets is not a tag id"))
                    })
                })
                .collect()
        })
        .collect()
}

/// A boolean column's filter, as a list sends it: the values a row may hold, repeated for both. A
/// lone empty value means neither box is checked, which matches no row; absence means no filter.
///
/// For `#[serde(default, deserialize_with = "deserialize_bool_values")]` on an
/// `Option<Vec<bool>>`.
pub fn deserialize_bool_values<'de, D>(deserializer: D) -> Result<Option<Vec<bool>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let Some(values) = Option::<Vec<String>>::deserialize(deserializer)? else {
        return Ok(None);
    };
    values
        .iter()
        .filter(|value| !value.is_empty())
        .map(|value| match value.as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            other => Err(serde::de::Error::custom(format!(
                "\"{other}\" is not true or false"
            ))),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

/// Each value's serde name: the text a column holding it stores, for filtering on it.
pub fn serde_names<V: Serialize>(values: &[V]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            serde_json::to_value(value)
                .ok()?
                .as_str()
                .map(str::to_string)
        })
        .collect()
}

/// Rows on the requested sites the caller can see, or on every site they can see when none is
/// requested. A requested site outside the caller's access narrows the result to nothing; it never
/// widens it.
pub fn on_requested_sites<T: Storable>(
    filter: StorableFilter<T>,
    requested: Option<&[Uuid]>,
    user_site_ids: &[Uuid],
) -> StorableFilter<T> {
    match requested {
        Some(requested) => {
            let accessible: Vec<Uuid> = requested
                .iter()
                .copied()
                .filter(|id| user_site_ids.contains(id))
                .collect();
            filter.site_ids(&accessible)
        }
        None => filter.site_ids(user_site_ids),
    }
}

/// The narrowing every taggable, searchable list shares: rows holding any of `tag_ids`, rows
/// holding a tag from each of `tag_sets`, and rows matching `search` through the entity's search
/// predicates. Server-side because the list is paginated: narrowing the loaded page instead would
/// miss matches on every other page.
pub fn narrow_by_tags_and_search<T: Entity>(
    filter: StorableFilter<T>,
    tag_ids: Option<&[Uuid]>,
    tag_sets: Option<&[String]>,
    search: Option<&str>,
) -> Result<StorableFilter<T>, ApiError> {
    let filter = match tag_ids {
        Some(tag_ids) if !tag_ids.is_empty() => filter.has_any_tags(tag_ids, T::ENTITY_TYPE),
        _ => filter,
    };
    let filter = filter.has_tags_from_each(&parse_tag_sets(tag_sets)?, T::ENTITY_TYPE);
    Ok(match search {
        Some(search) if !search.trim().is_empty() => filter.text_search(search),
        _ => filter,
    })
}

#[cfg(test)]
mod tag_sets_tests {
    use super::*;

    #[test]
    fn each_entry_becomes_one_set_and_a_bad_id_is_refused() {
        let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let sets = vec![format!("{a},{b}"), String::new(), c.to_string()];
        assert_eq!(
            parse_tag_sets(Some(&sets)).unwrap(),
            vec![vec![a, b], vec![c]]
        );
        assert!(parse_tag_sets(None).unwrap().is_empty());
        assert!(parse_tag_sets(Some(&["nope".to_string()])).is_err());
    }
}

/// A check for list filter queries: every parameter a list endpoint documents must narrow the
/// rows it returns. The query type is what both the list and its CSV export deserialize, so a
/// documented parameter nothing applies is a filter the API advertises and silently ignores.
#[cfg(test)]
pub(crate) mod narrowing_check {
    use crate::server::shared::extractors::Query;
    use crate::server::shared::storage::filter::StorableFilter;
    use crate::server::shared::storage::traits::Storable;
    use axum::extract::FromRequestParts;
    use serde::de::DeserializeOwned;
    use std::collections::BTreeSet;
    use utoipa::IntoParams;

    async fn parse<Q: DeserializeOwned + Send + 'static>(query: &str) -> Q {
        let request = axum::http::Request::builder()
            .uri(format!("/list?{query}"))
            .body(())
            .expect("request builds");
        let (mut parts, ()) = request.into_parts();
        let Query(parsed) = Query::<Q>::from_request_parts(&mut parts, &())
            .await
            .unwrap_or_else(|_| panic!("`{query}` deserialises"));
        parsed
    }

    /// `samples` gives each narrowing parameter a valid value; `not_narrowing` names the ones that
    /// page, order, shape the response or are applied elsewhere. Together they must cover every
    /// documented parameter, so a new one fails here until it is applied or explained.
    pub async fn assert_every_param_narrows<Q, T>(
        narrow: impl Fn(&Q) -> StorableFilter<T>,
        samples: &[(&str, &str)],
        not_narrowing: &[&str],
    ) where
        Q: DeserializeOwned + IntoParams + Send + 'static,
        T: Storable,
    {
        let documented: BTreeSet<String> = Q::into_params(|| None)
            .into_iter()
            .map(|param| param.name)
            .collect();
        let covered: BTreeSet<String> = samples
            .iter()
            .map(|(name, _)| name.to_string())
            .chain(not_narrowing.iter().map(|name| name.to_string()))
            .collect();
        assert_eq!(
            documented, covered,
            "every documented parameter needs a sample value or a reason it does not narrow"
        );

        let unfiltered = narrow(&parse::<Q>("").await);
        let baseline = (unfiltered.to_where_clause(), bound_lists(&unfiltered));
        for (name, value) in samples {
            let filter = narrow(&parse::<Q>(&format!("{name}={value}")).await);
            assert_ne!(
                (filter.to_where_clause(), bound_lists(&filter)),
                baseline,
                "`{name}` is documented but does not narrow the rows"
            );
        }
    }

    /// The bind count and each bound list's contents. A list parameter binds one array, so a
    /// filter that swaps the caller's site list for a narrower one changes only the array.
    fn bound_lists<T: Storable>(filter: &StorableFilter<T>) -> (usize, Vec<String>) {
        use crate::server::shared::storage::traits::SqlValue;
        let lists = filter
            .values()
            .iter()
            .filter_map(|value| match value {
                SqlValue::UuidArray(ids) => Some(format!("{ids:?}")),
                SqlValue::StringArray(items) => Some(format!("{items:?}")),
                SqlValue::I32Array(items) => Some(format!("{items:?}")),
                _ => None,
            })
            .collect();
        (filter.values().len(), lists)
    }
}
