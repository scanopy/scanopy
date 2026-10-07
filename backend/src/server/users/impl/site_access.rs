use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, postgres::PgRow};
use std::fmt::Display;
use uuid::Uuid;

use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::storage::{
    filter::StorableFilter,
    generic::GenericPostgresStorage,
    lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
    traits::{SqlValue, Storable, Storage},
};

/// The base data for a UserSiteAccess junction record
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct UserSiteAccessBase {
    pub user_id: Uuid,
    pub site_id: Uuid,
}

impl UserSiteAccessBase {
    pub fn new(user_id: Uuid, site_id: Uuid) -> Self {
        Self { user_id, site_id }
    }
}

/// A junction record linking a user to a site they have access to
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct UserSiteAccess {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub base: UserSiteAccessBase,
}

impl UserSiteAccess {
    pub fn new(base: UserSiteAccessBase) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            base,
        }
    }

    pub fn user_id(&self) -> Uuid {
        self.base.user_id
    }

    pub fn site_id(&self) -> Uuid {
        self.base.site_id
    }
}

impl Display for UserSiteAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UserSiteAccess(user={}, site={})",
            self.base.user_id, self.base.site_id
        )
    }
}

impl Storable for UserSiteAccess {
    const HAS_SCD2: bool = false;
    type BaseData = UserSiteAccessBase;

    fn table_name() -> &'static str {
        "user_site_access"
    }

    fn new(base: Self::BaseData) -> Self {
        UserSiteAccess::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["id", "user_id", "site_id", "created_at"],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.user_id),
                SqlValue::Uuid(self.base.site_id),
                SqlValue::Timestamp(self.created_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(UserSiteAccess {
            id: row.get("id"),
            created_at: row.get("created_at"),
            base: UserSiteAccessBase {
                user_id: row.get("user_id"),
                site_id: row.get("site_id"),
            },
        })
    }
}

/// Storage operations for user_site_access junction table.
/// Manages the site access list for each user.
pub struct UserSiteAccessStorage {
    storage: GenericPostgresStorage<UserSiteAccess>,
}

impl UserSiteAccessStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get all site IDs for a single user
    pub async fn get_for_user(&self, user_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter = StorableFilter::<UserSiteAccess>::new_from_user_id(user_id);
        let access_records = self.storage.get_all(filter).await?;
        Ok(access_records.iter().map(|a| a.site_id()).collect())
    }

    /// Get all user IDs with explicit access to a single site.
    pub async fn get_user_ids_for_site(&self, site_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter = StorableFilter::<UserSiteAccess>::new_from_site_ids(&[*site_id]);
        let access_records = self.storage.get_all(filter).await?;
        Ok(access_records.iter().map(|a| a.user_id()).collect())
    }

    /// Get site IDs for multiple users (batch loading)
    pub async fn get_for_users(&self, user_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if user_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = StorableFilter::<UserSiteAccess>::new_from_user_ids(user_ids);
        let access_records = self.storage.get_all(filter).await?;

        let mut result: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for access in access_records {
            result
                .entry(access.user_id())
                .or_default()
                .push(access.site_id());
        }

        Ok(result)
    }

    /// Save site IDs for a user (replaces all existing).
    /// Uses a transaction to ensure atomicity - if any insert fails, the delete is rolled back.
    pub async fn save_for_user(&self, user_id: &Uuid, site_ids: &[Uuid]) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;
        // Serialize concurrent delete-all + re-insert syncs for one user.
        tx.lock(
            LockKey::JunctionSync {
                parent: EntityDiscriminants::User,
                parent_id: *user_id,
            },
            DEFAULT_LOCK_TIMEOUT,
        )
        .await?;

        // Delete existing access for this user
        let filter = StorableFilter::<UserSiteAccess>::new_from_user_id(user_id);
        tx.delete_by_filter(filter).await?;

        // Insert new access records
        for site_id in site_ids {
            let access = UserSiteAccess::new(UserSiteAccessBase::new(*user_id, *site_id));
            tx.create(&access).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Delete all site access for a user
    pub async fn delete_for_user(&self, user_id: &Uuid) -> Result<()> {
        let filter = StorableFilter::<UserSiteAccess>::new_from_user_id(user_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }

    /// Add a single site to a user's access
    pub async fn add_site(&self, user_id: &Uuid, site_id: &Uuid) -> Result<()> {
        let access = UserSiteAccess::new(UserSiteAccessBase::new(*user_id, *site_id));
        // The storage will handle the unique constraint violation gracefully
        let _ = self.storage.create(&access).await;
        Ok(())
    }

    /// Remove a single site from a user's access
    pub async fn remove_site(&self, user_id: &Uuid, site_id: &Uuid) -> Result<()> {
        let filter = StorableFilter::<UserSiteAccess>::new_from_user_id(user_id)
            .uuid_column("site_id", site_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }
}
