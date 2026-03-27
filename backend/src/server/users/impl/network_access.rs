use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, postgres::PgRow};
use std::fmt::Display;
use uuid::Uuid;

use crate::server::shared::storage::{
    filter::StorableFilter,
    generic::GenericPostgresStorage,
    traits::{SqlValue, Storable, Storage},
};

/// The base data for a UserNetworkAccess junction record
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct UserNetworkAccessBase {
    pub user_id: Uuid,
    pub network_id: Uuid,
}

impl UserNetworkAccessBase {
    pub fn new(user_id: Uuid, network_id: Uuid) -> Self {
        Self {
            user_id,
            network_id,
        }
    }
}

/// A junction record linking a user to a network they have access to
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct UserNetworkAccess {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub base: UserNetworkAccessBase,
}

impl UserNetworkAccess {
    pub fn new(base: UserNetworkAccessBase) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            base,
        }
    }

    pub fn user_id(&self) -> Uuid {
        self.base.user_id
    }

    pub fn network_id(&self) -> Uuid {
        self.base.network_id
    }
}

impl Display for UserNetworkAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UserNetworkAccess(user={}, network={})",
            self.base.user_id, self.base.network_id
        )
    }
}

impl Storable for UserNetworkAccess {
    type BaseData = UserNetworkAccessBase;

    fn table_name() -> &'static str {
        "user_network_access"
    }

    fn new(base: Self::BaseData) -> Self {
        UserNetworkAccess::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["id", "user_id", "network_id", "created_at"],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.user_id),
                SqlValue::Uuid(self.base.network_id),
                SqlValue::Timestamp(self.created_at),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(UserNetworkAccess {
            id: row.get("id"),
            created_at: row.get("created_at"),
            base: UserNetworkAccessBase {
                user_id: row.get("user_id"),
                network_id: row.get("network_id"),
            },
        })
    }
}

/// Storage operations for user_network_access junction table.
/// Manages the network access list for each user.
pub struct UserNetworkAccessStorage {
    storage: GenericPostgresStorage<UserNetworkAccess>,
}

impl UserNetworkAccessStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get all network IDs for a single user
    pub async fn get_for_user(&self, user_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter = StorableFilter::<UserNetworkAccess>::new_from_user_id(user_id);
        let access_records = self.storage.get_all(filter).await?;
        Ok(access_records.iter().map(|a| a.network_id()).collect())
    }

    /// Get network IDs for multiple users (batch loading)
    pub async fn get_for_users(&self, user_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if user_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let filter = StorableFilter::<UserNetworkAccess>::new_from_user_ids(user_ids);
        let access_records = self.storage.get_all(filter).await?;

        let mut result: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for access in access_records {
            result
                .entry(access.user_id())
                .or_default()
                .push(access.network_id());
        }

        Ok(result)
    }

    /// Save network IDs for a user (replaces all existing).
    /// Uses a transaction to ensure atomicity - if any insert fails, the delete is rolled back.
    pub async fn save_for_user(&self, user_id: &Uuid, network_ids: &[Uuid]) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;

        // Delete existing access for this user
        let filter = StorableFilter::<UserNetworkAccess>::new_from_user_id(user_id);
        tx.delete_by_filter(filter).await?;

        // Insert new access records
        for network_id in network_ids {
            let access = UserNetworkAccess::new(UserNetworkAccessBase::new(*user_id, *network_id));
            tx.create(&access).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Delete all network access for a user
    pub async fn delete_for_user(&self, user_id: &Uuid) -> Result<()> {
        let filter = StorableFilter::<UserNetworkAccess>::new_from_user_id(user_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }

    /// Add a single network to a user's access
    pub async fn add_network(&self, user_id: &Uuid, network_id: &Uuid) -> Result<()> {
        let access = UserNetworkAccess::new(UserNetworkAccessBase::new(*user_id, *network_id));
        // The storage will handle the unique constraint violation gracefully
        let _ = self.storage.create(&access).await;
        Ok(())
    }

    /// Remove a single network from a user's access
    pub async fn remove_network(&self, user_id: &Uuid, network_id: &Uuid) -> Result<()> {
        let filter = StorableFilter::<UserNetworkAccess>::new_from_user_id(user_id)
            .uuid_column("network_id", network_id);
        self.storage.delete_by_filter(filter).await?;
        Ok(())
    }
}
