use std::fmt::Display;

use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::PgRow;
use uuid::Uuid;

use crate::server::shared::storage::traits::{SqlValue, Storable};

/// One server version this database has run, recorded by the first container
/// to start on it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReleaseAnnouncement {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub base: ReleaseAnnouncementBase,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReleaseAnnouncementBase {
    /// The server version, as semver text. Unique.
    pub version: String,
    /// When the release email for this version went out, to everyone or to
    /// licence owners only. `None` when this version sent none.
    pub email_sent_at: Option<DateTime<Utc>>,
}

impl Display for ReleaseAnnouncement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "release announcement {}: {}", self.base.version, self.id)
    }
}

impl Storable for ReleaseAnnouncement {
    const HAS_SCD2: bool = false;
    type BaseData = ReleaseAnnouncementBase;

    fn table_name() -> &'static str {
        "release_announcements"
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
                ReleaseAnnouncementBase {
                    version,
                    email_sent_at,
                },
        } = self.clone();

        Ok((
            vec!["id", "version", "email_sent_at", "created_at", "updated_at"],
            vec![
                SqlValue::Uuid(id),
                SqlValue::String(version),
                SqlValue::OptionTimestamp(email_sent_at),
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
            base: ReleaseAnnouncementBase {
                version: row.get("version"),
                email_sent_at: row.get("email_sent_at"),
            },
        })
    }
}
