use std::fmt::Display;

use crate::server::{
    config::AppState,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        handlers::{
            list::{ListOrdering, PaginatedList},
            ordering::TagGroupOrdering,
            query::narrow_by_tags_and_search,
            traits::{CrudHandlers, ListScope, base_list_filter},
        },
        storage::filter::{Junction, StorableFilter},
        types::api::ApiResult,
    },
    sites::{
        handlers::{SiteListQuery, SiteOrderField},
        service::SiteService,
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::postgres::PgRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::product_area::ProductArea;
use crate::server::shared::storage::traits::{Entity, SqlValue, Storable};

#[derive(
    Debug, Clone, Serialize, Deserialize, Validate, PartialEq, Eq, Hash, Default, ToSchema,
)]
pub struct SiteBase {
    /// Human-facing name for this site.
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    /// The organization that owns this record.
    pub organization_id: Uuid,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    /// Credential IDs assigned to this site (hydrated from the junction table, so always present
    /// in a response). On create, omitting it assigns the organization's site-wide default
    /// credentials and a list, even an empty one, is used exactly. On update, omitting it leaves
    /// the assignments unchanged.
    #[serde(default)]
    pub credential_ids: Option<Vec<Uuid>>,
    /// How long a discovery-managed entity on this site may go unobserved
    /// before it reads as stale. `None` = unset; callers resolve the effective
    /// value through [`Site::stale_after`], never by reading this directly.
    ///
    /// Site-scoped because staleness is only meaningful relative to scan
    /// cadence, and cadence is a property of a site's discoveries.
    #[validate(range(min = 1, max = 87_600))]
    #[serde(default)]
    #[schema(required)]
    pub stale_after_hours: Option<i64>,
}

/// Effective staleness threshold when a site has not set one: 28 days.
///
/// Deliberately generous — a laptop away for a few weeks, a seasonally-powered
/// lab box or a host behind a scan that has been failing quietly should not be
/// declared stale before a human would agree. Sites that scan aggressively
/// can tighten it per-site.
///
/// Lives here rather than as a DDL default so it can change without a
/// migration, and so `NULL` keeps meaning "unset" rather than "explicitly 28
/// days" — the distinction a future per-org or per-use-case default needs.
pub const DEFAULT_STALE_AFTER_HOURS: i64 = 24 * 28;

impl SiteBase {
    pub fn new(organization_id: Uuid) -> Self {
        Self {
            name: "My Site".to_string(),
            organization_id,
            tags: Vec::new(),
            credential_ids: None,
            stale_after_hours: None,
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
#[schema(example = crate::server::shared::types::examples::site)]
pub struct Site {
    /// Server-assigned unique identifier.
    #[serde(default)]
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When this record was first created.
    #[serde(default)]
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    #[serde(default)]
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    /// `stale_after_hours` with the server's default already applied.
    ///
    /// Computed, never stored (excluded from `to_params`). Published so the
    /// frontend derives staleness from the *same* number the digest uses rather
    /// than re-declaring the default in TypeScript, where the two could drift
    /// and a host could read stale in the app but current in the digest email.
    #[serde(default)]
    #[schema(read_only)]
    pub effective_stale_after_hours: i64,
    #[serde(flatten)]
    #[validate(nested)]
    pub base: SiteBase,
}

impl Site {
    /// Effective staleness window for this site, falling back to
    /// [`DEFAULT_STALE_AFTER_HOURS`] when unset. The single place the fallback
    /// is applied — callers must not read `base.stale_after_hours` directly.
    pub fn stale_after(&self) -> chrono::Duration {
        // Reads the base field rather than `effective_stale_after_hours` so an
        // in-memory Site built without going through `from_row` still
        // resolves correctly.
        chrono::Duration::hours(
            self.base
                .stale_after_hours
                .unwrap_or(DEFAULT_STALE_AFTER_HOURS),
        )
    }

    /// Instant before which a `last_seen_at` on this site counts as stale.
    /// `reference` is `now()` for the UI/API read path and the discovery
    /// session's `finished_at` for the digest, so both surfaces derive the same
    /// verdict from the same rule.
    pub fn stale_cutoff(&self, reference: DateTime<Utc>) -> DateTime<Utc> {
        reference - self.stale_after()
    }
}

impl Display for Site {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.base.name, self.id)
    }
}

/// The credentials assigned to each site.
const SITE_CREDENTIALS: Junction = Junction {
    table: "site_credentials",
    own_column: "site_id",
    other_column: "credential_id",
    has_history: false,
};

impl SiteListQuery {
    /// Everything the sites list narrows by: the caller's sites, staleness window, assigned
    /// credentials, tags and search.
    pub fn narrow(&self, scope: &ListScope) -> ApiResult<StorableFilter<Site>> {
        let filter = narrow_by_tags_and_search(
            base_list_filter::<Site>(scope, self),
            self.tag_ids.as_deref(),
            self.tag_sets.as_deref(),
            self.search.as_deref(),
        )?;
        Ok(match &self.credential_ids {
            Some(ids) if !ids.is_empty() => filter.linked_to(SITE_CREDENTIALS, ids),
            _ => filter,
        })
    }
}

#[async_trait::async_trait]
impl CrudHandlers for Site {
    type Service = SiteService;
    type FilterQuery = SiteListQuery;
    type OrderField = SiteOrderField;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.site_service
    }

    fn search_order() -> Option<Self::OrderField> {
        Some(SiteOrderField::Name)
    }

    async fn list_filter(
        _state: &AppState,
        query: &SiteListQuery,
        scope: &ListScope,
    ) -> ApiResult<StorableFilter<Site>> {
        query.narrow(scope)
    }
}

#[async_trait::async_trait]
impl PaginatedList for Site {
    type Item = Site;

    fn ordering(query: &SiteListQuery) -> ListOrdering<SiteOrderField> {
        ListOrdering {
            group_by: query.group_by,
            order_by: query.order_by,
            direction: query.order_direction,
            tag_group: TagGroupOrdering {
                group_by: query.group_by_tag_group,
                order_by: query.order_by_tag_group,
            },
        }
    }

    /// Each site with the credentials assigned to it.
    async fn items(
        state: &AppState,
        mut rows: Vec<Site>,
        _query: &SiteListQuery,
    ) -> ApiResult<Vec<Site>> {
        let ids: Vec<Uuid> = rows.iter().map(|s| s.id).collect();
        let mut credentials = state
            .services
            .credential_service
            .get_credential_ids_for_sites(&ids)
            .await?;
        for site in &mut rows {
            site.base.credential_ids = Some(credentials.remove(&site.id).unwrap_or_default());
        }
        Ok(rows)
    }
}

impl crate::server::shared::events::log::EntityLog for Site {}

impl ChangeTriggersTopologyStaleness<Site> for Site {
    fn triggers_staleness(&self, _other: Option<Site>) -> bool {
        false
    }
}

impl Storable for Site {
    const HAS_SCD2: bool = false;
    type BaseData = SiteBase;

    fn table_name() -> &'static str {
        "sites"
    }

    fn search_predicates() -> &'static [&'static str] {
        &[
            "sites.name ILIKE {}",
            "EXISTS (SELECT 1 FROM daemons d WHERE d.site_id = sites.id AND d.name ILIKE {})",
            "EXISTS (SELECT 1 FROM subnets s WHERE s.site_id = sites.id \
             AND s.valid_to IS NULL AND s.name ILIKE {})",
            "EXISTS (SELECT 1 FROM vlans v WHERE v.site_id = sites.id \
             AND v.valid_to IS NULL AND v.name ILIKE {})",
        ]
    }

    fn new(base: Self::BaseData) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            effective_stale_after_hours: base
                .stale_after_hours
                .unwrap_or(DEFAULT_STALE_AFTER_HOURS),
            base,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        let Self {
            id,
            created_at,
            updated_at,
            // Derived from `stale_after_hours` for the API response; never stored.
            effective_stale_after_hours: _,
            base:
                Self::BaseData {
                    name,
                    organization_id,
                    tags: _,           // Stored in entity_tags junction table
                    credential_ids: _, // Stored in site_credentials junction table
                    stale_after_hours,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "created_at",
                "updated_at",
                "name",
                "organization_id",
                "stale_after_hours",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
                SqlValue::String(name),
                SqlValue::Uuid(organization_id),
                SqlValue::OptionalI64(stale_after_hours),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let stale_after_hours: Option<i64> = row.get("stale_after_hours");
        Ok(Site {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            effective_stale_after_hours: stale_after_hours.unwrap_or(DEFAULT_STALE_AFTER_HOURS),
            base: SiteBase {
                name: row.get("name"),
                organization_id: row.get("organization_id"),
                tags: Vec::new(),     // Hydrated from entity_tags junction table
                credential_ids: None, // Hydrated from site_credentials junction table
                stale_after_hours,
            },
        })
    }
}

impl Entity for Site {
    fn id(&self) -> Uuid {
        self.id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    fn set_id(&mut self, id: Uuid) {
        self.id = id;
    }

    fn set_created_at(&mut self, time: DateTime<Utc>) {
        self.created_at = time;
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Site
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Site";
    const ENTITY_NAME_PLURAL: &'static str = "Sites";
    const ENTITY_DESCRIPTION: &'static str = "Site containers. Top-level organizational unit that contains subnets, hosts, and other entities.";

    fn product_area() -> ProductArea {
        ProductArea::Inventory
    }

    /// A site is its own scope, so reaching a site by id takes access to that site.
    fn site_id(&self) -> Option<Uuid> {
        Some(self.id)
    }

    /// A site is scoped to itself, not to a site column: its rows belong to the organization.
    fn is_site_keyed() -> bool {
        false
    }

    fn organization_id(&self) -> Option<Uuid> {
        Some(self.base.organization_id)
    }

    fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    fn set_updated_at(&mut self, time: DateTime<Utc>) {
        self.updated_at = time;
    }

    fn get_tags(&self) -> Option<&Vec<Uuid>> {
        Some(&self.base.tags)
    }

    fn set_tags(&mut self, tags: Vec<Uuid>) {
        self.base.tags = tags;
    }
}
