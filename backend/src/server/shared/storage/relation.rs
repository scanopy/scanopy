//! How one stored row points at another, declared once per edge.
//!
//! A [`Relation`] names the column on row type `T` that holds another row's id, and whether `T`
//! can exist without that row. Three consumers read the same const:
//!
//! - the field registry's hops (`Hop::parent`, `Hop::children` in `shared/entity_fields.rs`)
//! - storage's set operations, which load or replace every row of `T` that points at one id
//!   ([`GenericPostgresStorage::get_for`], [`GenericPostgresStorage::replace_for`] and the rest)
//! - the topology bundle's cascade, which drops rows whose owner left the bundle
//!
//! A row type lists its relations in [`Storable::relations`]. A junction row lists both ends:
//! `subnet_vlans` belongs to its subnet and to its VLAN alike, and either end can key a set
//! operation. The consts live in `shared/relations.rs`.

use std::{collections::HashMap, fmt::Display};

use anyhow::Result;
use sqlx::Postgres;
use uuid::Uuid;

use super::{
    filter::StorableFilter,
    generic::GenericPostgresStorage,
    lock::{DEFAULT_LOCK_TIMEOUT, LockKey, xact_lock},
    snapshot::Snapshotable,
    traits::{Storable, Storage},
};
use crate::server::shared::entities::EntityDiscriminants;

/// A key on a serialized object that holds the id, or ids, of a `to` entity. Untyped, so one
/// const serves every entity that carries the key (`site_id`, `tags`). The registry's
/// `Hop::Parent` is built from one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reference {
    pub to: EntityDiscriminants,
    pub key: &'static str,
}

impl Reference {
    pub const fn new(to: EntityDiscriminants, key: &'static str) -> Self {
        Self { to, key }
    }
}

/// Whether a row can exist without the row it points at. Mirrors the column's foreign key:
/// `Owner` is `ON DELETE CASCADE`, `Reference` is `ON DELETE SET NULL` or no foreign key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    /// The row stays when the other row goes.
    Reference,
    /// The row goes with the other row: deleted with it, and left out of a topology that leaves
    /// it out.
    Owner,
    /// Deleted with the other row, but kept when a topology only hides it. A neighbour row's far
    /// end: the near port is still linked when the far host is dismissed.
    OwnerKeptWhenHidden,
}

/// Reads the id or ids a relation's column holds on a typed row.
pub enum Ids<T: 'static> {
    /// One id, or `None` when this row has none (a binding of the other type, a principal of
    /// another kind).
    One(fn(&T) -> Option<Uuid>),
    /// An array of ids.
    Many(fn(&T) -> &[Uuid]),
}

/// What a relation's column points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Entity(EntityDiscriminants),
    /// Any entity, named by the row's `entity_type` column (`entity_tags`).
    AnyEntity,
}

/// One edge from row type `T` to another row: the column on `T` holding the other row's id.
/// `T`'s table and entity type come from `T` itself.
pub struct Relation<T: 'static> {
    pub target: Target,
    pub column: &'static str,
    pub ownership: Ownership,
    pub ids: Ids<T>,
}

impl<T: 'static> Relation<T> {
    /// The edge from `T` through `reference`.
    pub const fn to(reference: Reference, ownership: Ownership, ids: Ids<T>) -> Self {
        Self {
            target: Target::Entity(reference.to),
            column: reference.key,
            ownership,
            ids,
        }
    }

    /// An edge whose column holds any entity's id, with the entity type in `entity_type`.
    pub const fn to_any_entity(column: &'static str, ownership: Ownership, ids: Ids<T>) -> Self {
        Self {
            target: Target::AnyEntity,
            column,
            ownership,
            ids,
        }
    }
}

impl<T: Storable> Relation<T> {
    /// The registry's view of this edge. Fails to compile in a const for an `AnyEntity` relation,
    /// which names no single entity.
    pub const fn reference(&self) -> Reference {
        match self.target {
            Target::Entity(to) => Reference {
                to,
                key: self.column,
            },
            Target::AnyEntity => panic!("an AnyEntity relation has no single target"),
        }
    }

    /// The ids this relation's column holds on `row`.
    pub fn ids_of(&self, row: &T) -> Vec<Uuid> {
        match self.ids {
            Ids::One(read) => read(row).into_iter().collect(),
            Ids::Many(read) => read(row).to_vec(),
        }
    }

    /// The rows of `T` that point at `id`, for a relation with one target entity.
    ///
    /// # Panics
    /// On an `AnyEntity` relation: use [`Self::at_entity`], which names the entity type.
    pub fn at(&'static self, id: Uuid) -> RelationKey<T> {
        match self.target {
            Target::Entity(entity_type) => RelationKey {
                relation: self,
                entity_type,
                id,
            },
            Target::AnyEntity => panic!(
                "{}.{} points at any entity: call at_entity",
                T::table_name(),
                self.column
            ),
        }
    }

    /// The rows of `T` that point at `id` of `entity_type`, for an `AnyEntity` relation.
    ///
    /// # Panics
    /// On a relation with one target entity: use [`Self::at`].
    pub fn at_entity(&'static self, entity_type: EntityDiscriminants, id: Uuid) -> RelationKey<T> {
        assert!(
            self.target == Target::AnyEntity,
            "{}.{} has one target entity: call at",
            T::table_name(),
            self.column
        );
        RelationKey {
            relation: self,
            entity_type,
            id,
        }
    }

    fn filter_many(&self, ids: &[Uuid]) -> StorableFilter<T> {
        StorableFilter::<T>::new_from_uuids_column(self.column, ids)
    }
}

/// The set of `T` rows that point at one row through one relation.
#[derive(Clone, Copy)]
pub struct RelationKey<T: 'static> {
    pub relation: &'static Relation<T>,
    pub entity_type: EntityDiscriminants,
    pub id: Uuid,
}

impl<T: Storable> RelationKey<T> {
    /// Every row in the set, live or closed.
    pub fn filter(&self) -> StorableFilter<T> {
        let filter = StorableFilter::<T>::new_from_uuid_column(self.relation.column, &self.id);
        match self.relation.target {
            Target::Entity(_) => filter,
            Target::AnyEntity => filter.entity_type(&self.entity_type),
        }
    }

    /// Writes to one set serialize on this, so concurrent replaces can't interleave into a
    /// duplicate or merged set.
    pub fn lock_key(&self) -> LockKey {
        LockKey::JunctionSync {
            parent: self.entity_type,
            parent_id: self.id,
        }
    }
}

/// Only live rows for a type with SCD2 columns. Closed copies stay out of current-state reads.
fn live_only<T: Storable>(filter: StorableFilter<T>) -> StorableFilter<T> {
    if T::HAS_SCD2 { filter.live() } else { filter }
}

impl<T: Storable + Display> GenericPostgresStorage<T> {
    /// The live rows in `key`'s set.
    pub async fn get_for(&self, key: &RelationKey<T>) -> Result<Vec<T>> {
        self.get_all(live_only(key.filter())).await
    }

    /// The live rows pointing at each of `ids` through `relation`, grouped by id.
    pub async fn get_for_many(
        &self,
        relation: &'static Relation<T>,
        ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<T>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut grouped: HashMap<Uuid, Vec<T>> = HashMap::new();
        for row in self.get_all(live_only(relation.filter_many(ids))).await? {
            for id in relation.ids_of(&row) {
                grouped.entry(id).or_default().push(row.clone());
            }
        }
        Ok(grouped)
    }

    /// Replace every row in `key`'s set with `rows`, in one transaction holding the set's lock.
    pub async fn replace_for(&self, key: &RelationKey<T>, rows: &[T]) -> Result<Vec<T>> {
        let mut tx = self.begin_transaction().await?;
        tx.lock(key.lock_key(), DEFAULT_LOCK_TIMEOUT).await?;
        tx.delete_by_filter(key.filter()).await?;
        let mut saved = Vec::with_capacity(rows.len());
        for row in rows {
            saved.push(tx.create(row).await?);
        }
        tx.commit().await?;
        Ok(saved)
    }

    /// [`Self::replace_for`] inside the caller's transaction, which holds the set's lock until it
    /// commits or rolls back.
    pub async fn replace_for_in_tx(
        &self,
        key: &RelationKey<T>,
        rows: &[T],
        tx: &mut sqlx::Transaction<'_, Postgres>,
    ) -> Result<Vec<T>> {
        xact_lock(tx, key.lock_key(), DEFAULT_LOCK_TIMEOUT).await?;
        Self::delete_by_filter_in_tx(key.filter(), tx).await?;
        let mut saved = Vec::with_capacity(rows.len());
        for row in rows {
            saved.push(Self::create_with_executor(row, &mut **tx).await?);
        }
        Ok(saved)
    }

    /// Add `row` to `key`'s set, keeping the rest, under the lock a replace holds.
    pub async fn add_for(&self, key: &RelationKey<T>, row: &T) -> Result<T> {
        let mut tx = self.begin_transaction().await?;
        tx.lock(key.lock_key(), DEFAULT_LOCK_TIMEOUT).await?;
        let saved = tx.create(row).await?;
        tx.commit().await?;
        Ok(saved)
    }

    /// Delete every row in `key`'s set.
    pub async fn delete_for(&self, key: &RelationKey<T>) -> Result<usize> {
        self.delete_by_filter(key.filter()).await
    }
}

impl<T: Snapshotable + Display> GenericPostgresStorage<T> {
    /// Make `member_ids` the live members of `key`'s set, keeping history: a live row whose
    /// member left the set is soft-closed (`valid_to = now`), a new member gets a new live row
    /// from `new_row`, and a row whose member stays is left alone, id included.
    ///
    /// The read-diff-write runs in one transaction holding the set's lock, so concurrent syncs
    /// can't interleave into duplicate or merged sets.
    pub async fn sync_members_for(
        &self,
        key: &RelationKey<T>,
        member_ids: &[Uuid],
        member_of: impl Fn(&T) -> Uuid,
        new_row: impl Fn(Uuid) -> T,
    ) -> Result<()> {
        let mut tx = self.begin_transaction().await?;
        tx.lock(key.lock_key(), DEFAULT_LOCK_TIMEOUT).await?;

        let existing = tx.get_all(key.filter().live()).await?;
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
        let mut rows = self.get_all(filter.live()).await?;
        if rows.is_empty() {
            return Ok(0);
        }
        let now = crate::server::shared::clock::now();
        for row in rows.iter_mut() {
            row.set_valid_to(Some(now));
        }
        self.update_many(&rows).await?;
        Ok(rows.len())
    }
}

#[cfg(test)]
mod tests;
