use std::collections::HashMap;
use std::fmt::Display;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::storage::{
    filter::StorableFilter,
    generic::GenericPostgresStorage,
    lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
    traits::{SqlValue, Storable, Storage},
};

/// The base data for a UserApiKeySiteAccess junction record
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct UserApiKeySiteAccessBase {
    pub api_key_id: Uuid,
    pub site_id: Uuid,
}

impl UserApiKeySiteAccessBase {
    pub fn new(api_key_id: Uuid, site_id: Uuid) -> Self {
        Self {
            api_key_id,
            site_id,
        }
    }
}

/// A junction record linking a user API key to a site it has access to
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct UserApiKeySiteAccess {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub base: UserApiKeySiteAccessBase,
}

impl UserApiKeySiteAccess {
    pub fn new(base: UserApiKeySiteAccessBase) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            base,
        }
    }

    pub fn api_key_id(&self) -> Uuid {
        self.base.api_key_id
    }

    pub fn site_id(&self) -> Uuid {
        self.base.site_id
    }
}

impl Display for UserApiKeySiteAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UserApiKeySiteAccess(api_key={}, site={})",
            self.base.api_key_id, self.base.site_id
        )
    }
}

impl Storable for UserApiKeySiteAccess {
    const HAS_SCD2: bool = false;
    type BaseData = UserApiKeySiteAccessBase;

    fn table_name() -> &'static str {
        "user_api_key_site_access"
    }

    fn new(base: Self::BaseData) -> Self {
        UserApiKeySiteAccess::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["id", "api_key_id", "site_id", "created_at"],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.api_key_id),
                SqlValue::Uuid(self.base.site_id),
                SqlValue::Timestamp(self.created_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(UserApiKeySiteAccess {
            id: row.get("id"),
            created_at: row.get("created_at"),
            base: UserApiKeySiteAccessBase {
                api_key_id: row.get("api_key_id"),
                site_id: row.get("site_id"),
            },
        })
    }
}

/// Storage operations for user_api_key_site_access junction table.
/// Manages the site access list for each user API key.
pub struct UserApiKeySiteAccessStorage {
    storage: GenericPostgresStorage<UserApiKeySiteAccess>,
}

impl UserApiKeySiteAccessStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get all site IDs for a single API key
    pub async fn get_for_key(&self, api_key_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter =
            StorableFilter::<UserApiKeySiteAccess>::new_from_uuid_column("api_key_id", api_key_id);
        let access_records = self.storage.get_all(filter).await?;
        Ok(access_records.iter().map(|a| a.site_id()).collect())
    }

    /// Get site IDs for multiple API keys (batch loading)
    pub async fn get_for_keys(&self, api_key_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if api_key_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = StorableFilter::<UserApiKeySiteAccess>::new_from_uuids_column(
            "api_key_id",
            api_key_ids,
        );
        let access_records = self.storage.get_all(filter).await?;

        let mut result: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for access in access_records {
            result
                .entry(access.api_key_id())
                .or_default()
                .push(access.site_id());
        }

        Ok(result)
    }

    /// Save site IDs for an API key (replaces all existing).
    /// Uses a transaction to ensure atomicity - if any insert fails, the delete is rolled back.
    pub async fn save_for_key(&self, api_key_id: &Uuid, site_ids: &[Uuid]) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;
        // Serialize concurrent delete-all + re-insert syncs for one key.
        tx.lock(
            LockKey::JunctionSync {
                parent: EntityDiscriminants::UserApiKey,
                parent_id: *api_key_id,
            },
            DEFAULT_LOCK_TIMEOUT,
        )
        .await?;

        // Delete existing access for this key
        let filter =
            StorableFilter::<UserApiKeySiteAccess>::new_from_uuid_column("api_key_id", api_key_id);
        tx.delete_by_filter(filter).await?;

        // Insert new access records
        for site_id in site_ids {
            let access =
                UserApiKeySiteAccess::new(UserApiKeySiteAccessBase::new(*api_key_id, *site_id));
            tx.create(&access).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Delete all site access for an API key
    pub async fn delete_for_key(&self, api_key_id: &Uuid) -> Result<()> {
        let filter =
            StorableFilter::<UserApiKeySiteAccess>::new_from_uuid_column("api_key_id", api_key_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }

    /// Add a single site to an API key's access
    pub async fn add_site(&self, api_key_id: &Uuid, site_id: &Uuid) -> Result<()> {
        let access =
            UserApiKeySiteAccess::new(UserApiKeySiteAccessBase::new(*api_key_id, *site_id));
        // The storage will handle the unique constraint violation gracefully
        let _ = self.storage.create(&access).await;
        Ok(())
    }

    /// Remove a single site from an API key's access
    pub async fn remove_site(&self, api_key_id: &Uuid, site_id: &Uuid) -> Result<()> {
        let filter =
            StorableFilter::<UserApiKeySiteAccess>::new_from_uuid_column("api_key_id", api_key_id)
                .uuid_column("site_id", site_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }
}
