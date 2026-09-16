use std::{collections::HashMap, fmt::Display};

use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use super::{generic::GenericPostgresStorage, traits::Storable};

/// Trait for entities that are children of a parent entity and stored in a separate table.
/// Extends Storable to reuse all the standard CRUD infrastructure while adding
/// parent-scoped batch operations.
///
/// The key addition is the "replace all" pattern: when saving via `save_for_parent`,
/// all existing children for that parent are deleted and the new set is inserted.
pub trait ChildStorableEntity: Storable {
    /// The column name for the parent foreign key (e.g., "host_id", "service_id")
    fn parent_column() -> &'static str;

    /// Get the parent ID for this entity
    fn parent_id(&self) -> Uuid;
}

/// Generic storage implementation for child entities.
/// Wraps GenericPostgresStorage and adds parent-scoped batch operations.
pub struct GenericChildStorage<T: ChildStorableEntity + Display> {
    pool: PgPool,
    inner: GenericPostgresStorage<T>,
}

impl<T: ChildStorableEntity + Display> GenericChildStorage<T> {
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: GenericPostgresStorage::new(pool.clone()),
            pool,
        }
    }

    /// Get the inner storage for standard CRUD operations
    pub fn inner(&self) -> &GenericPostgresStorage<T> {
        &self.inner
    }

    /// The `valid_to IS NULL` clause, or nothing for children that carry no SCD2 columns.
    ///
    /// This used to be appended unconditionally, on the assumption that every
    /// `ChildStorableEntity` was also `Snapshotable`. GH #701 ended that: raw LLDP/CDP
    /// evidence in `interface_neighbor_candidates` is deliberately not historised (the
    /// resolved adjacencies in `interface_neighbor_interfaces` / `_hosts` still are), so the
    /// clause named a column that does not exist and every child read for an interface with
    /// neighbour evidence failed. Gate on the entity's own declaration rather than on which
    /// types happen to implement the trait today.
    fn live_row_filter() -> &'static str {
        if T::HAS_SCD2 {
            " AND valid_to IS NULL"
        } else {
            ""
        }
    }

    /// Get all children for a single parent.
    ///
    /// For SCD2 entities this returns only live (`valid_to IS NULL`) rows, which is
    /// required for correctness: natural-key reconciliation and current-state reads must
    /// not see closed historical copies. Non-SCD2 children have no such column and are
    /// returned whole — see [`Self::live_row_filter`].
    pub async fn get_for_parent(&self, parent_id: &Uuid) -> Result<Vec<T>> {
        let query_str = format!(
            "SELECT * FROM {} WHERE {} = $1{}",
            T::table_name(),
            T::parent_column(),
            Self::live_row_filter()
        );

        let rows = sqlx::query(&query_str)
            .bind(parent_id)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(|row| T::from_row(&row)).collect()
    }

    /// Get children for multiple parents (batch loading).
    /// Returns a map of parent_id -> children.
    /// Live-only for SCD2 entities — see `get_for_parent` doc.
    pub async fn get_for_parents(&self, parent_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<T>>> {
        if parent_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let query_str = format!(
            "SELECT * FROM {} WHERE {} = ANY($1){}",
            T::table_name(),
            T::parent_column(),
            Self::live_row_filter()
        );

        let rows = sqlx::query(&query_str)
            .bind(parent_ids)
            .fetch_all(&self.pool)
            .await?;

        let mut result: HashMap<Uuid, Vec<T>> = HashMap::new();
        for row in rows {
            let entity = T::from_row(&row)?;
            let parent_id = entity.parent_id();
            result.entry(parent_id).or_default().push(entity);
        }

        Ok(result)
    }
}

/// Async trait for child storage operations (for use with dependency injection)
#[async_trait]
pub trait ChildStorage<T: ChildStorableEntity + Display>: Send + Sync {
    async fn get_for_parent(&self, parent_id: &Uuid) -> Result<Vec<T>>;
    async fn get_for_parents(&self, parent_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<T>>>;
    /// Save children for a parent, returning the saved entities with actual IDs
    async fn save_for_parent(&self, parent_id: &Uuid, children: &[T]) -> Result<Vec<T>>;
    async fn delete_for_parent(&self, parent_id: &Uuid) -> Result<()>;
}

#[async_trait]
impl<T: ChildStorableEntity + Display> ChildStorage<T> for GenericChildStorage<T> {
    async fn get_for_parent(&self, parent_id: &Uuid) -> Result<Vec<T>> {
        self.get_for_parent(parent_id).await
    }

    async fn get_for_parents(&self, parent_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<T>>> {
        self.get_for_parents(parent_ids).await
    }

    async fn save_for_parent(&self, parent_id: &Uuid, children: &[T]) -> Result<Vec<T>> {
        self.save_for_parent(parent_id, children).await
    }

    async fn delete_for_parent(&self, parent_id: &Uuid) -> Result<()> {
        self.delete_for_parent(parent_id).await
    }
}
