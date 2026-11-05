//! Server instances per online license key.
//!
//! A self-hosted server's instance is its database: the first container to
//! start creates one [`ServerInstance`] row, and every container and replica
//! on that database sends its id with each check-in. The cloud records one
//! [`LicenseInstance`] per id it hears from, and counts the ones heard from
//! within `LICENSE_SILENCE_THRESHOLD_DAYS` against the organization's limits:
//! one for production servers and one for servers started with
//! `SCANOPY_NON_PRODUCTION`.

pub mod service;

use std::fmt::Display;
use std::net::IpAddr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::postgres::PgRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::server::license::online::InstanceUsage;
use crate::server::organizations::r#impl::base::LICENSE_SILENCE_THRESHOLD_DAYS;
use crate::server::shared::storage::traits::{SqlValue, Storable};

/// This database's instance id, held by self-hosted servers. One row per
/// database.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerInstance {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Display for ServerInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "server instance {}", self.id)
    }
}

impl Storable for ServerInstance {
    const HAS_SCD2: bool = false;
    type BaseData = ();

    fn table_name() -> &'static str {
        "server_instance"
    }

    fn new(_base: Self::BaseData) -> Self {
        let now = crate::server::shared::clock::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
        }
    }

    fn get_base(&self) -> Self::BaseData {}

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        Ok((
            vec!["id", "created_at", "updated_at"],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Timestamp(self.created_at),
                SqlValue::Timestamp(self.updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(Self {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

/// A server instance that has checked in with an organization's online key,
/// as the cloud records it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LicenseInstance {
    /// Id of the cloud's record for this instance.
    #[schema(read_only, required)]
    pub id: Uuid,
    /// When the cloud created the record.
    #[schema(read_only, required)]
    pub created_at: DateTime<Utc>,
    /// When the cloud last changed the record.
    #[schema(read_only, required)]
    pub updated_at: DateTime<Utc>,
    #[serde(flatten)]
    pub base: LicenseInstanceBase,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LicenseInstanceBase {
    /// Organization whose online key the instance checks in with.
    pub organization_id: Uuid,
    /// The id the server's database holds. `None` on the one row shared by
    /// servers too old to send it.
    pub instance_id: Option<Uuid>,
    /// Version the latest check-in reported, as semver text.
    pub server_version: Option<String>,
    /// Source address the cloud observed on the latest check-in. The server
    /// never sends an address; behind NAT several servers share one.
    #[schema(value_type = Option<String>, example = "203.0.113.7")]
    pub last_seen_ip: Option<IpAddr>,
    /// Whether the latest check-in came from a server started with
    /// `SCANOPY_NON_PRODUCTION`. Production and non-production servers take
    /// slots from separate limits. `None` when that check-in came from a
    /// server older than the setting: it counts as production and is never
    /// refused.
    pub non_production: Option<bool>,
    /// First check-in.
    pub first_seen_at: DateTime<Utc>,
    /// Latest check-in.
    pub last_seen_at: DateTime<Utc>,
    /// When the instance last started counting in its class: its first
    /// check-in, its first after a silence, or a change of class. Slots go
    /// to active instances in this order, so a server coming back never
    /// displaces one that held the slot meanwhile.
    pub active_since: DateTime<Utc>,
}

/// How many production and non-production servers one online key covers.
/// `None` is no cap.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InstanceLimits {
    pub production: Option<u32>,
    pub non_production: Option<u32>,
}

impl InstanceLimits {
    /// The limit for one class of server.
    pub fn for_class(&self, non_production: bool) -> Option<u32> {
        if non_production {
            self.non_production
        } else {
            self.production
        }
    }
}

impl LicenseInstance {
    /// Whether the instance counts against the limit at `now`: it checked in
    /// within `LICENSE_SILENCE_THRESHOLD_DAYS`, the span after which a server
    /// that stopped checking in counts as gone.
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        now - self.base.last_seen_at < chrono::Duration::days(LICENSE_SILENCE_THRESHOLD_DAYS)
    }

    /// The class whose slots the instance takes. Servers that don't say are
    /// production.
    pub fn is_non_production(&self) -> bool {
        self.base.non_production.unwrap_or(false)
    }
}

/// The active instances of one class at `now`, in the order they take slots.
fn active_in_class(
    instances: &[LicenseInstance],
    non_production: bool,
    now: DateTime<Utc>,
) -> Vec<&LicenseInstance> {
    let mut active: Vec<&LicenseInstance> = instances
        .iter()
        .filter(|i| i.is_non_production() == non_production && i.is_active_at(now))
        .collect();
    active.sort_by_key(|i| (i.base.active_since, i.id));
    active
}

/// How many of `instances` count against each limit at `now`.
pub fn usage(
    instances: &[LicenseInstance],
    limits: InstanceLimits,
    now: DateTime<Utc>,
) -> InstanceUsage {
    let count = |non_production| {
        u32::try_from(active_in_class(instances, non_production, now).len()).unwrap_or(u32::MAX)
    };
    InstanceUsage {
        active: count(false),
        limit: limits.production,
        non_production_active: count(true),
        non_production_limit: limits.non_production,
    }
}

/// Whether `instance` holds one of its class's slots at `now`. Active
/// instances take a class's slots in `active_since` order, so a server that
/// arrives past the limit is the one refused. `None` is no cap.
pub fn holds_slot(
    instances: &[LicenseInstance],
    instance: &LicenseInstance,
    limits: InstanceLimits,
    now: DateTime<Utc>,
) -> bool {
    let Some(limit) = limits.for_class(instance.is_non_production()) else {
        return true;
    };
    active_in_class(instances, instance.is_non_production(), now)
        .iter()
        .take(usize::try_from(limit).unwrap_or(usize::MAX))
        .any(|i| i.id == instance.id)
}

/// Whether `instance` is refused an entitlement at `now`: it is past its
/// class's limit and its latest check-in said which class it is. A server
/// too old to say is counted but never refused.
pub fn is_refused(
    instances: &[LicenseInstance],
    instance: &LicenseInstance,
    limits: InstanceLimits,
    now: DateTime<Utc>,
) -> bool {
    instance.base.non_production.is_some() && !holds_slot(instances, instance, limits, now)
}

impl Display for LicenseInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "license instance {}", self.id)
    }
}

impl Storable for LicenseInstance {
    const HAS_SCD2: bool = false;
    type BaseData = LicenseInstanceBase;

    fn table_name() -> &'static str {
        "license_instances"
    }

    fn new(base: Self::BaseData) -> Self {
        let now = crate::server::shared::clock::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
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
            base:
                LicenseInstanceBase {
                    organization_id,
                    instance_id,
                    server_version,
                    last_seen_ip,
                    non_production,
                    first_seen_at,
                    last_seen_at,
                    active_since,
                },
        } = self.clone();

        Ok((
            vec![
                "id",
                "organization_id",
                "instance_id",
                "server_version",
                "last_seen_ip",
                "non_production",
                "first_seen_at",
                "last_seen_at",
                "active_since",
                "created_at",
                "updated_at",
            ],
            vec![
                SqlValue::Uuid(id),
                SqlValue::Uuid(organization_id),
                SqlValue::OptionalUuid(instance_id),
                SqlValue::OptionalString(server_version),
                SqlValue::OptionalIpAddr(last_seen_ip),
                SqlValue::OptionalBool(non_production),
                SqlValue::Timestamp(first_seen_at),
                SqlValue::Timestamp(last_seen_at),
                SqlValue::Timestamp(active_since),
                SqlValue::Timestamp(created_at),
                SqlValue::Timestamp(updated_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        Ok(Self {
            id: row.get("id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            base: LicenseInstanceBase {
                organization_id: row.get("organization_id"),
                instance_id: row.get("instance_id"),
                server_version: row.get("server_version"),
                last_seen_ip: row
                    .get::<Option<ipnetwork::IpNetwork>, _>("last_seen_ip")
                    .map(|network| network.ip()),
                non_production: row.get("non_production"),
                first_seen_at: row.get("first_seen_at"),
                last_seen_at: row.get("last_seen_at"),
                active_since: row.get("active_since"),
            },
        })
    }
}
