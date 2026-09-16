use std::fmt::Display;

use crate::server::{
    config::AppState,
    networks::service::NetworkService,
    shared::{
        entities::{ChangeTriggersTopologyStaleness, EntityDiscriminants},
        handlers::{query::NoFilterQuery, traits::CrudHandlers},
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::postgres::PgRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::server::shared::entity_metadata::EntityCategory;
use crate::server::shared::storage::traits::{Entity, SqlValue, Storable};

/// CSV row representation for Network export
#[derive(Serialize)]
pub struct NetworkCsvRow {
    pub id: Uuid,
    pub name: String,
    pub organization_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, Validate, PartialEq, Eq, Hash, Default, ToSchema,
)]
pub struct NetworkBase {
    /// Human-facing name for this network.
    #[validate(length(min = 0, max = 100))]
    pub name: String,
    /// The organization that owns this record.
    pub organization_id: Uuid,
    /// Tags assigned to this entity.
    #[serde(default)]
    #[schema(required)]
    pub tags: Vec<Uuid>,
    /// Credential IDs associated with this network (hydrated from junction table).
    #[serde(default)]
    #[schema(required)]
    pub credential_ids: Vec<Uuid>,
    /// How long a discovery-managed entity on this network may go unobserved
    /// before it reads as stale. `None` = unset; callers resolve the effective
    /// value through [`Network::stale_after`], never by reading this directly.
    ///
    /// Network-scoped because staleness is only meaningful relative to scan
    /// cadence, and cadence is a property of a network's discoveries.
    #[validate(range(min = 1, max = 87_600))]
    #[serde(default)]
    #[schema(required)]
    pub stale_after_hours: Option<i64>,
}

/// Effective staleness threshold when a network has not set one: 28 days.
///
/// Deliberately generous — a laptop away for a few weeks, a seasonally-powered
/// lab box or a host behind a scan that has been failing quietly should not be
/// declared stale before a human would agree. Networks that scan aggressively
/// can tighten it per-network.
///
/// Lives here rather than as a DDL default so it can change without a
/// migration, and so `NULL` keeps meaning "unset" rather than "explicitly 28
/// days" — the distinction a future per-org or per-use-case default needs.
pub const DEFAULT_STALE_AFTER_HOURS: i64 = 24 * 28;

impl NetworkBase {
    pub fn new(organization_id: Uuid) -> Self {
        Self {
            name: "My Network".to_string(),
            organization_id,
            tags: Vec::new(),
            credential_ids: Vec::new(),
            stale_after_hours: None,
        }
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, ToSchema, Validate,
)]
#[schema(example = crate::server::shared::types::examples::network)]
pub struct Network {
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
    pub base: NetworkBase,
}

impl Network {
    /// Effective staleness window for this network, falling back to
    /// [`DEFAULT_STALE_AFTER_HOURS`] when unset. The single place the fallback
    /// is applied — callers must not read `base.stale_after_hours` directly.
    pub fn stale_after(&self) -> chrono::Duration {
        // Reads the base field rather than `effective_stale_after_hours` so an
        // in-memory Network built without going through `from_row` still
        // resolves correctly.
        chrono::Duration::hours(
            self.base
                .stale_after_hours
                .unwrap_or(DEFAULT_STALE_AFTER_HOURS),
        )
    }

    /// Instant before which a `last_seen_at` on this network counts as stale.
    /// `reference` is `now()` for the UI/API read path and the discovery
    /// session's `finished_at` for the digest, so both surfaces derive the same
    /// verdict from the same rule.
    pub fn stale_cutoff(&self, reference: DateTime<Utc>) -> DateTime<Utc> {
        reference - self.stale_after()
    }
}

impl Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.base.name, self.id)
    }
}

impl CrudHandlers for Network {
    type Service = NetworkService;
    type FilterQuery = NoFilterQuery;

    fn get_service(state: &AppState) -> &Self::Service {
        &state.services.network_service
    }
}

impl ChangeTriggersTopologyStaleness<Network> for Network {
    fn triggers_staleness(&self, _other: Option<Network>) -> bool {
        false
    }
}

impl Storable for Network {
    const HAS_SCD2: bool = false;
    type BaseData = NetworkBase;

    fn table_name() -> &'static str {
        "networks"
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
                    credential_ids: _, // Stored in network_credentials junction table
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
        Ok(Network {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            effective_stale_after_hours: stale_after_hours.unwrap_or(DEFAULT_STALE_AFTER_HOURS),
            base: NetworkBase {
                name: row.get("name"),
                organization_id: row.get("organization_id"),
                tags: Vec::new(), // Hydrated from entity_tags junction table
                credential_ids: Vec::new(), // Hydrated from network_credentials junction table
                stale_after_hours,
            },
        })
    }
}

impl Entity for Network {
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

    type CsvRow = NetworkCsvRow;

    fn to_csv_row(&self) -> Self::CsvRow {
        NetworkCsvRow {
            id: self.id,
            name: self.base.name.clone(),
            organization_id: self.base.organization_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    fn entity_type() -> EntityDiscriminants {
        EntityDiscriminants::Network
    }

    const ENTITY_NAME_SINGULAR: &'static str = "Network";
    const ENTITY_NAME_PLURAL: &'static str = "Networks";
    const ENTITY_DESCRIPTION: &'static str = "Network containers. Top-level organizational unit that contains subnets, hosts, and other entities.";

    fn entity_category() -> EntityCategory {
        EntityCategory::NetworkInfrastructure
    }

    fn network_id(&self) -> Option<Uuid> {
        None
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
