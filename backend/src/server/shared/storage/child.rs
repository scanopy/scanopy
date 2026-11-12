use std::{collections::HashMap, fmt::Display};

use anyhow::Result;
use sqlx::{PgPool, Postgres};
use uuid::Uuid;

use super::{
    filter::StorableFilter,
    generic::GenericPostgresStorage,
    lock::{DEFAULT_LOCK_TIMEOUT, LockKey, xact_lock},
    snapshot::Snapshotable,
    traits::{Storable, Storage},
};
use crate::server::shared::entities::EntityDiscriminants;

/// The parent a set of child rows belongs to: which entity, and which one of it.
///
/// The entity type names the `LockKey::JunctionSync` a write to the set serializes on, and for a
/// table whose parent column is shared by several entity types (`entity_tags`) it also scopes the
/// rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentRef {
    pub entity_type: EntityDiscriminants,
    pub id: Uuid,
}

impl ParentRef {
    pub fn new(entity_type: EntityDiscriminants, id: Uuid) -> Self {
        Self { entity_type, id }
    }

    fn lock_key(&self) -> LockKey {
        LockKey::JunctionSync {
            parent: self.entity_type,
            parent_id: self.id,
        }
    }
}

/// A row stored in its own table under a parent: an entity's children (ports, bindings), a
/// junction table's links, or the non-entity rows a parent owns (neighbour evidence, discovery
/// warnings).
pub trait ChildStorableEntity: Storable {
    /// The column name for the parent foreign key (e.g., "host_id", "service_id")
    fn parent_column() -> &'static str;

    /// Get the parent ID for this entity
    fn parent_id(&self) -> Uuid;

    /// The rows belonging to `parent`. Matches on the parent column alone, which is enough for
    /// every table whose parent column names a single entity type.
    fn parent_filter(parent: &ParentRef) -> StorableFilter<Self> {
        StorableFilter::<Self>::new_from_uuid_column(Self::parent_column(), &parent.id)
    }
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
            .persistent(false)
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
            .persistent(false)
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

    /// Replace the rows matching `filter` with `rows`, in one transaction holding `lock`'s
    /// `JunctionSync` lock so concurrent replaces of one set can't interleave into a duplicate
    /// or merged set.
    ///
    /// For a junction written from its other end, whose rows the parent column doesn't select.
    /// Writes from the declared parent use [`Self::save_for_parent`].
    pub async fn replace_matching(
        &self,
        lock: &ParentRef,
        filter: StorableFilter<T>,
        rows: &[T],
    ) -> Result<Vec<T>> {
        let mut tx = self.inner.begin_transaction().await?;
        tx.lock(lock.lock_key(), DEFAULT_LOCK_TIMEOUT).await?;
        tx.delete_by_filter(filter).await?;
        let mut saved = Vec::with_capacity(rows.len());
        for row in rows {
            saved.push(tx.create(row).await?);
        }
        tx.commit().await?;
        Ok(saved)
    }

    /// Replace every row of `parent` with `rows`. See [`Self::replace_matching`].
    pub async fn save_for_parent(&self, parent: &ParentRef, rows: &[T]) -> Result<Vec<T>> {
        self.replace_matching(parent, T::parent_filter(parent), rows)
            .await
    }

    /// [`Self::save_for_parent`] inside the caller's transaction, which holds the `JunctionSync`
    /// lock until it commits or rolls back.
    pub async fn save_for_parent_in_tx(
        &self,
        parent: &ParentRef,
        rows: &[T],
        tx: &mut sqlx::Transaction<'_, Postgres>,
    ) -> Result<Vec<T>> {
        xact_lock(tx, parent.lock_key(), DEFAULT_LOCK_TIMEOUT).await?;
        GenericPostgresStorage::<T>::delete_by_filter_in_tx(T::parent_filter(parent), tx).await?;
        let mut saved = Vec::with_capacity(rows.len());
        for row in rows {
            saved.push(GenericPostgresStorage::<T>::create_with_executor(row, &mut **tx).await?);
        }
        Ok(saved)
    }

    /// Add `row` to `parent`'s set, keeping the rest, under the same lock a replace holds.
    pub async fn add_for_parent(&self, parent: &ParentRef, row: &T) -> Result<T> {
        let mut tx = self.inner.begin_transaction().await?;
        tx.lock(parent.lock_key(), DEFAULT_LOCK_TIMEOUT).await?;
        let saved = tx.create(row).await?;
        tx.commit().await?;
        Ok(saved)
    }

    /// Delete every row of `parent`.
    pub async fn delete_for_parent(&self, parent: &ParentRef) -> Result<usize> {
        self.inner.delete_by_filter(T::parent_filter(parent)).await
    }
}

impl<T: ChildStorableEntity + Snapshotable + Display> GenericChildStorage<T> {
    /// Make `member_ids` the live members of `parent`, keeping history: a live row whose member
    /// left the set is soft-closed (`valid_to = now`), a new member gets a new live row from
    /// `new_row`, and a row whose member stays is left alone, id included.
    ///
    /// The read-diff-write runs in one transaction holding the parent's `JunctionSync` lock, so
    /// concurrent syncs can't interleave into duplicate or merged sets.
    pub async fn sync_for_parent(
        &self,
        parent: &ParentRef,
        member_ids: &[Uuid],
        member_of: impl Fn(&T) -> Uuid,
        new_row: impl Fn(Uuid) -> T,
    ) -> Result<()> {
        let mut tx = self.inner.begin_transaction().await?;
        tx.lock(parent.lock_key(), DEFAULT_LOCK_TIMEOUT).await?;

        let existing = tx.get_all(T::parent_filter(parent).live()).await?;
        let wanted: std::collections::HashSet<Uuid> = member_ids.iter().copied().collect();
        let held: std::collections::HashSet<Uuid> = existing.iter().map(&member_of).collect();

        let now = crate::server::shared::clock::now();
        for row in existing
            .iter()
            .filter(|row| !wanted.contains(&member_of(row)))
        {
            let mut closed = row.clone();
            closed.set_valid_to(Some(now));
            tx.update(&mut closed).await?;
        }
        for member_id in member_ids.iter().filter(|id| !held.contains(id)) {
            tx.create(&new_row(*member_id)).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Soft-close every live row matching `filter` (`valid_to = now`), keeping it for as-of
    /// reads. Returns how many rows were closed.
    pub async fn close_matching(&self, filter: StorableFilter<T>) -> Result<usize> {
        let mut rows = self.inner.get_all(filter.live()).await?;
        if rows.is_empty() {
            return Ok(0);
        }
        let now = crate::server::shared::clock::now();
        for row in rows.iter_mut() {
            row.set_valid_to(Some(now));
        }
        self.inner.update_many(&rows).await?;
        Ok(rows.len())
    }
}
