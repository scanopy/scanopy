use crate::server::shared::{
    storage::{
        filter::StorableFilter,
        lock::{LockError, LockKey, SessionLockGuard},
        pg_value::{Bound, PgJson, PgText},
        traits::{PaginatedResult, SqlValue, Storable, Storage, Unique},
    },
    types::api::ValidationError,
};
use async_trait::async_trait;
use sqlx::{Executor, PgPool, Postgres, postgres::PgArguments};
use std::{fmt::Display, marker::PhantomData};
use uuid::Uuid;

// Re-export for convenience
pub use sqlx::postgres::PgConnection;

pub struct GenericPostgresStorage<T: Storable> {
    pool: PgPool,
    _phantom: PhantomData<T>,
}

/// Whether a SQLSTATE names a data exception (class 22) rather than a database problem.
///
/// Split out from [`GenericPostgresStorage::is_data_exception`] so the classification can be
/// tested against the codes themselves — `sqlx::postgres::PgDatabaseError` cannot be constructed
/// outside sqlx, and the rule is the part worth pinning down.
fn is_data_exception_code(code: Option<&str>) -> bool {
    // Five characters, first two "22". The length check matters: SQLSTATE is fixed-width, and a
    // prefix test alone would also catch a hypothetical longer vendor code starting "22".
    code.is_some_and(|code| code.len() == 5 && code.starts_with("22"))
}

impl<T: Storable> GenericPostgresStorage<T>
where
    T: Display,
{
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            _phantom: PhantomData,
        }
    }

    /// Access the underlying connection pool. Lets callers (e.g.
    /// `SnapshotService`) open one transaction and run multi-table
    /// operations through the static `*_in_tx` methods on the typed
    /// `GenericPostgresStorage<T>` namespaces.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Generate INSERT query dynamically
    fn build_insert_query(columns: &[&str]) -> String {
        let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();

        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            T::table_name(),
            columns.join(", "),
            placeholders.join(", ")
        )
    }

    /// Generate multi-row INSERT query: INSERT INTO table (cols) VALUES ($1,$2), ($3,$4), ...
    fn build_bulk_insert_query(columns: &[&str], row_count: usize) -> String {
        let cols_per_row = columns.len();
        let value_groups: Vec<String> = (0..row_count)
            .map(|row| {
                let placeholders: Vec<String> = (1..=cols_per_row)
                    .map(|col| format!("${}", row * cols_per_row + col))
                    .collect();
                format!("({})", placeholders.join(", "))
            })
            .collect();

        format!(
            "INSERT INTO {} ({}) VALUES {}",
            T::table_name(),
            columns.join(", "),
            value_groups.join(", ")
        )
    }

    /// Generate UPDATE query dynamically
    fn build_update_query(columns: &[&str]) -> String {
        let set_clauses: Vec<String> = columns
            .iter()
            .enumerate()
            .skip(1) // Skip 'id' column
            .map(|(i, col)| format!("{} = ${}", col, i + 1))
            .collect();

        format!(
            "UPDATE {} SET {} WHERE id = $1",
            T::table_name(),
            set_clauses.join(", ")
        )
    }

    /// Whether a database error is about the *value* rather than the database.
    ///
    /// SQLSTATE class 22 is "data exception" — `22021` a character the encoding cannot represent,
    /// `22P05` an unsupported Unicode escape in `jsonb`, `22001` a string too long for its column,
    /// `22003` a number out of range, `22007`/`22P02` an unparseable literal. Every one of them
    /// means "this row is wrong", never "the connection or the database is unhealthy".
    ///
    /// The distinction decides blast radius. Callers that write an entity and its children treat
    /// a `ValidationError` as "skip this child, keep the rest" and anything else as systemic and
    /// worth abandoning the whole entity for — see `HostService::create_with_children`. Before
    /// this, one NUL byte on one switch port took the entire host down, and with it every
    /// interface the L2 topology needed (GH #668). Classifying by class rather than by the one
    /// code we happened to hit means the next encoding surprise degrades the same way instead of
    /// finding a new way to lose a device.
    fn is_data_exception(db_err: &dyn sqlx::error::DatabaseError) -> bool {
        is_data_exception_code(db_err.code().as_deref())
    }

    /// Convert a unique constraint violation into a user-friendly message
    fn friendly_unique_violation_message(constraint: Option<&str>) -> String {
        match constraint {
            // ports(host_id, port_number, protocol)
            Some(c) if c.contains("ports") => {
                "A port with this number and protocol already exists on this host".to_string()
            }
            // ip_addresses(host_id, subnet_id, ip_address)
            Some(c) if c.contains("ip_addresses") => {
                "An ip_address with this IP address already exists on this host".to_string()
            }
            // tags(organization_id, name)
            Some(c) if c.contains("tags") => "A tag with this name already exists".to_string(),
            // group_bindings(group_id, binding_id)
            Some(c) if c.contains("group_bindings") => {
                "This binding already exists in the group".to_string()
            }
            // user_site_access(user_id, site_id)
            Some(c) if c.contains("user_site_access") => {
                "This user already has access to this site".to_string()
            }
            // users - email or name
            Some(c) if c.contains("users") && c.contains("email") => {
                "A user with this email already exists".to_string()
            }
            Some(c) if c.contains("users") && c.contains("name") => {
                "A user with this name already exists".to_string()
            }
            Some(c) if c.contains("users") && c.contains("oidc") => {
                "This identity provider account is already linked to another user".to_string()
            }
            // api_keys(key)
            Some(c) if c.contains("api_keys") => "This API key already exists".to_string(),
            Some(c) => {
                format!("A record with these values already exists ({})", c)
            }
            None => "A record with these values already exists".to_string(),
        }
    }

    /// Bind one `SqlValue` to the query.
    ///
    /// The domain-to-wire mapping lives in [`SqlValue::to_bound`], which has no access to
    /// `query` — that is what keeps [`Bound`]'s sanitised text and JSON variants the only route
    /// to a `text`/`jsonb` column. See `pg_value.rs` for why the invariant is enforced there
    /// rather than trusted here.
    fn bind_value<'q>(
        query: sqlx::query::Query<'q, Postgres, PgArguments>,
        value: &'q SqlValue,
    ) -> Result<sqlx::query::Query<'q, Postgres, PgArguments>, anyhow::Error> {
        let bound = value.to_bound()?;

        if bound.stripped() {
            // Never routine: some ingestion path handed us a character PostgreSQL cannot store.
            // Naming the table is the actionable part — it points at the writer to fix, rather
            // than leaving this layer to compensate silently forever. See GH #668.
            tracing::warn!(
                table = T::table_name(),
                "Removed NUL bytes from a value before writing it; the source integration should \
                 not be producing them"
            );
        }

        Ok(match bound {
            Bound::Text(v) => query.bind(v.into_inner()),
            Bound::OptText(v) => query.bind(v.map(PgText::into_inner)),
            Bound::TextArray(v) => {
                query.bind(v.into_iter().map(PgText::into_inner).collect::<Vec<_>>())
            }
            Bound::OptTextArray(v) => query.bind(v.map(|items| {
                items
                    .into_iter()
                    .map(PgText::into_inner)
                    .collect::<Vec<_>>()
            })),
            Bound::Json(v) => query.bind(v.into_inner()),
            Bound::OptJson(v) => query.bind(v.map(PgJson::into_inner)),
            Bound::Uuid(v) => query.bind(v),
            Bound::OptUuid(v) => query.bind(v),
            Bound::UuidArray(v) => query.bind(v),
            Bound::OptUuidArray(v) => query.bind(v),
            Bound::I32(v) => query.bind(v),
            Bound::OptI32(v) => query.bind(v),
            Bound::I64(v) => query.bind(v),
            Bound::OptI64(v) => query.bind(v),
            Bound::Bool(v) => query.bind(v),
            Bound::Timestamp(v) => query.bind(v),
            Bound::OptTimestamp(v) => query.bind(v),
            Bound::IpNet(v) => query.bind(v),
            Bound::OptIpNet(v) => query.bind(v),
            Bound::Mac(v) => query.bind(v),
            Bound::MacArray(v) => query.bind(v),
            Bound::OptMac(v) => query.bind(v),
        })
    }

    // =========================================================================
    // Internal executor-generic methods
    // These accept any sqlx Executor (pool or transaction) and contain the
    // actual implementation logic. Public methods delegate to these.
    // =========================================================================

    /// Internal: create entity with any executor
    pub async fn create_with_executor<'e, E>(entity: &T, executor: E) -> Result<T, anyhow::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let (columns, values) = entity.to_params()?;
        let query_str = Self::build_insert_query(&columns);

        let mut query = sqlx::query(&query_str);
        for value in &values {
            query = Self::bind_value(query, value)?;
        }

        match query.execute(executor).await {
            Ok(_) => {
                tracing::trace!("Created {}: {}", T::table_name(), entity);
                Ok(entity.clone())
            }
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                let friendly_msg = Self::friendly_unique_violation_message(db_err.constraint());
                Err(ValidationError::new(friendly_msg).into())
            }
            Err(sqlx::Error::Database(db_err)) if Self::is_data_exception(db_err.as_ref()) => {
                tracing::warn!(
                    table = T::table_name(),
                    code = db_err.code().as_deref(),
                    error = %db_err,
                    "Rejected a row PostgreSQL could not store; skipping it rather than failing \
                     everything written alongside it"
                );
                Err(ValidationError::new(format!(
                    "This record contains a value the database cannot store: {db_err}"
                ))
                .into())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Maximum bind parameters per PostgreSQL query
    const MAX_BIND_PARAMS: usize = 65535;

    /// Internal: bulk create entities with any executor.
    /// Chunks automatically to stay under PostgreSQL's bind parameter limit.
    pub async fn create_many_with_executor(
        entities: &[T],
        pool: &PgPool,
    ) -> Result<Vec<T>, anyhow::Error> {
        if entities.is_empty() {
            return Ok(vec![]);
        }

        // Get column count from first entity to calculate chunk size
        let (columns, _) = entities[0].to_params()?;
        let cols_per_row = columns.len();
        let chunk_size = Self::MAX_BIND_PARAMS / cols_per_row;

        for chunk in entities.chunks(chunk_size) {
            // Collect all params first to get owned column names
            let all_params: Vec<(Vec<&'static str>, Vec<SqlValue>)> = chunk
                .iter()
                .map(|e| e.to_params())
                .collect::<Result<_, _>>()?;

            let query_str = Self::build_bulk_insert_query(&all_params[0].0, chunk.len());
            let mut query = sqlx::query(&query_str);

            for (_, values) in &all_params {
                for value in values {
                    query = Self::bind_value(query, value)?;
                }
            }

            match query.execute(pool).await {
                Ok(_) => {
                    tracing::trace!("Bulk created {} {}s", chunk.len(), T::table_name());
                }
                Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                    let friendly_msg = Self::friendly_unique_violation_message(db_err.constraint());
                    let detail = db_err
                        .try_downcast_ref::<sqlx::postgres::PgDatabaseError>()
                        .and_then(|pg| pg.detail().map(|d| d.to_string()));
                    tracing::warn!(
                        table = T::table_name(),
                        constraint = db_err.constraint(),
                        detail = detail,
                        error = %db_err,
                        "Bulk insert unique violation"
                    );
                    return Err(ValidationError::new(friendly_msg).into());
                }
                Err(sqlx::Error::Database(db_err)) if Self::is_data_exception(db_err.as_ref()) => {
                    // Asymmetry worth knowing: a bulk insert fails as a chunk, so unlike the
                    // single-row path this loses every entity in the chunk, not just the offending
                    // one. Still a validation failure rather than a systemic one — the caller's
                    // recovery is the same, and splitting the chunk to isolate the bad row would
                    // add a retry path for a case the sanitiser in `pg_value.rs` already prevents.
                    tracing::warn!(
                        table = T::table_name(),
                        code = db_err.code().as_deref(),
                        chunk_len = chunk.len(),
                        error = %db_err,
                        "Bulk insert rejected: a value in this chunk cannot be stored"
                    );
                    return Err(ValidationError::new(format!(
                        "A record in this batch contains a value the database cannot store: \
                         {db_err}"
                    ))
                    .into());
                }
                Err(e) => return Err(e.into()),
            }
        }

        Ok(entities.to_vec())
    }

    /// Internal: delete by filter with any executor
    pub async fn delete_by_filter_with_executor<'e, E>(
        filter: StorableFilter<T>,
        executor: E,
    ) -> Result<usize, anyhow::Error>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let query_str = format!(
            "DELETE FROM {} {}",
            T::table_name(),
            filter.to_where_clause()
        );

        let mut query = sqlx::query(&query_str);
        for value in filter.values() {
            query = Self::bind_value(query, value)?;
        }

        let result = query.execute(executor).await?;
        let deleted_count = result.rows_affected() as usize;

        tracing::trace!("Deleted {} {}s by filter", deleted_count, T::table_name());

        Ok(deleted_count)
    }

    // =========================================================================
    // Transaction support
    // =========================================================================

    /// Acquire a session-scoped DB advisory lock via this storage's pool.
    /// See `shared/storage/lock.rs` — this keeps services off the raw pool.
    pub async fn session_lock(
        &self,
        key: LockKey,
        timeout: std::time::Duration,
    ) -> Result<SessionLockGuard, LockError> {
        super::lock::session_lock(&self.pool, key, timeout).await
    }

    /// Acquire several session-scoped DB advisory locks in canonical order.
    pub async fn session_lock_many(
        &self,
        keys: &[LockKey],
        timeout: std::time::Duration,
    ) -> Result<Vec<SessionLockGuard>, LockError> {
        super::lock::session_lock_many(&self.pool, keys, timeout).await
    }

    /// Begin a new transaction. Use the returned `StorageTransaction` for
    /// transactional operations, then call `commit()` to persist changes.
    /// If dropped without committing, the transaction is automatically rolled back.
    pub async fn begin_transaction(&self) -> Result<StorageTransaction<'_, T>, anyhow::Error> {
        let tx = self.pool.begin().await?;
        Ok(StorageTransaction {
            tx,
            _phantom: PhantomData,
        })
    }

    /// Fetch all rows matching `filter` within an externally-owned
    /// `sqlx::Transaction`. Mirrors the public `Storage::get_all` shape
    /// (default `created_at ASC` order) but lets `SnapshotService` share
    /// one transaction across many entity types.
    pub async fn get_all_in_tx(
        filter: StorableFilter<T>,
        tx: &mut sqlx::Transaction<'_, Postgres>,
    ) -> Result<Vec<T>, anyhow::Error> {
        let pagination_clause = filter.to_pagination_clause();
        let join_clause = filter.to_join_clause();

        let select = if filter.has_joins() {
            format!("{}.*", T::table_name())
        } else {
            "*".to_string()
        };

        let query_str = format!(
            "SELECT {} FROM {} {} {} ORDER BY created_at ASC {}",
            select,
            T::table_name(),
            join_clause,
            filter.to_where_clause(),
            pagination_clause
        );

        let mut query = sqlx::query(&query_str);
        for value in filter.values() {
            query = Self::bind_value(query, value)?;
        }

        let rows = query.fetch_all(&mut **tx).await?;
        rows.into_iter().map(|r| T::from_row(&r)).collect()
    }

    /// Bulk INSERT mirroring `create_many_with_executor`'s shape but bound
    /// to an externally-owned `sqlx::Transaction`. Same chunking around
    /// `MAX_BIND_PARAMS`. Used by `SnapshotService` so close-and-clone is
    /// atomic across all 12 site-scoped entity types.
    pub async fn create_many_in_tx(
        entities: &[T],
        tx: &mut sqlx::Transaction<'_, Postgres>,
    ) -> Result<Vec<T>, anyhow::Error> {
        if entities.is_empty() {
            return Ok(vec![]);
        }

        let (columns, _) = entities[0].to_params()?;
        let cols_per_row = columns.len();
        let chunk_size = Self::MAX_BIND_PARAMS / cols_per_row.max(1);

        for chunk in entities.chunks(chunk_size) {
            let all_params: Vec<(Vec<&'static str>, Vec<SqlValue>)> = chunk
                .iter()
                .map(|e| e.to_params())
                .collect::<Result<_, _>>()?;

            let query_str = Self::build_bulk_insert_query(&all_params[0].0, chunk.len());
            let mut query = sqlx::query(&query_str);

            for (_, values) in &all_params {
                for value in values {
                    query = Self::bind_value(query, value)?;
                }
            }

            query.execute(&mut **tx).await?;
        }

        Ok(entities.to_vec())
    }

    /// Bulk UPDATE within an externally-owned transaction. Same per-entity
    /// `to_params` → `UPDATE ... WHERE id = $1` shape as the public
    /// `Storage::update_many`. All updates run inside the supplied
    /// transaction; commit/rollback is the caller's responsibility.
    pub async fn update_many_in_tx(
        entities: &[T],
        tx: &mut sqlx::Transaction<'_, Postgres>,
    ) -> Result<Vec<T>, anyhow::Error> {
        if entities.is_empty() {
            return Ok(vec![]);
        }

        for entity in entities {
            let (columns, values) = entity.to_params()?;
            let query_str = Self::build_update_query(&columns);
            let mut query = sqlx::query(&query_str);
            for value in &values {
                query = Self::bind_value(query, value)?;
            }
            query.execute(&mut **tx).await?;
        }

        Ok(entities.to_vec())
    }
}

/// A transactional wrapper around storage operations.
/// Provides the same API as `GenericPostgresStorage` but executes within a transaction.
/// Must call `commit()` to persist changes; automatically rolls back on drop.
pub struct StorageTransaction<'a, T: Storable> {
    tx: sqlx::Transaction<'a, Postgres>,
    _phantom: PhantomData<T>,
}

impl<'a, T: Storable> StorageTransaction<'a, T>
where
    T: Display,
{
    /// Create an entity within the transaction
    pub async fn create(&mut self, entity: &T) -> Result<T, anyhow::Error> {
        GenericPostgresStorage::<T>::create_with_executor(entity, &mut *self.tx).await
    }

    /// Get entity by ID within the transaction (sees uncommitted writes
    /// from this transaction).
    pub async fn get_by_id(&mut self, id: &Uuid) -> Result<Option<T>, anyhow::Error> {
        let filter = StorableFilter::<T>::new_from_entity_id(id);
        let query_str = format!(
            "SELECT * FROM {} {}",
            T::table_name(),
            filter.to_where_clause()
        );
        let mut query = sqlx::query(&query_str);
        for value in filter.values() {
            query = GenericPostgresStorage::<T>::bind_value(query, value)?;
        }
        let row = query.fetch_optional(&mut *self.tx).await?;
        row.map(|r| T::from_row(&r)).transpose()
    }

    /// Get entity by ID within the transaction, taking a row-level
    /// `FOR UPDATE` lock: concurrent readers of the same row block until
    /// this transaction commits/rolls back, then re-read the updated row.
    /// Use for single-row read-modify-writes (counter increments, SCD2
    /// close-and-clone).
    pub async fn get_by_id_for_update(&mut self, id: &Uuid) -> Result<Option<T>, anyhow::Error> {
        let filter = StorableFilter::<T>::new_from_entity_id(id);
        let query_str = format!(
            "SELECT * FROM {} {} FOR UPDATE",
            T::table_name(),
            filter.to_where_clause()
        );
        let mut query = sqlx::query(&query_str);
        for value in filter.values() {
            query = GenericPostgresStorage::<T>::bind_value(query, value)?;
        }
        let row = query.fetch_optional(&mut *self.tx).await?;
        row.map(|r| T::from_row(&r)).transpose()
    }

    /// Get all entities matching the filter within the transaction (sees
    /// uncommitted writes from this transaction).
    pub async fn get_all(&mut self, filter: StorableFilter<T>) -> Result<Vec<T>, anyhow::Error> {
        GenericPostgresStorage::<T>::get_all_in_tx(filter, &mut self.tx).await
    }

    /// Acquire a transaction-scoped DB advisory lock (auto-released at
    /// commit/rollback). See `shared/storage/lock.rs`.
    pub async fn lock(
        &mut self,
        key: LockKey,
        timeout: std::time::Duration,
    ) -> Result<(), LockError> {
        super::lock::xact_lock(&mut self.tx, key, timeout).await
    }

    /// Update an entity within the transaction
    pub async fn update(&mut self, entity: &mut T) -> Result<T, anyhow::Error> {
        let (columns, values) = entity.to_params()?;
        let query_str = GenericPostgresStorage::<T>::build_update_query(&columns);
        let mut query = sqlx::query(&query_str);
        for value in &values {
            query = GenericPostgresStorage::<T>::bind_value(query, value)?;
        }
        query.execute(&mut *self.tx).await?;
        Ok(entity.clone())
    }

    /// Delete entities matching the filter within the transaction
    pub async fn delete_by_filter(
        &mut self,
        filter: StorableFilter<T>,
    ) -> Result<usize, anyhow::Error> {
        GenericPostgresStorage::<T>::delete_by_filter_with_executor(filter, &mut *self.tx).await
    }

    /// Commit the transaction, persisting all changes
    pub async fn commit(self) -> Result<(), anyhow::Error> {
        self.tx.commit().await?;
        Ok(())
    }

    /// Explicitly rollback the transaction (also happens automatically on drop)
    pub async fn rollback(self) -> Result<(), anyhow::Error> {
        self.tx.rollback().await?;
        Ok(())
    }
}

#[async_trait]
impl<T: Storable> Storage<T> for GenericPostgresStorage<T>
where
    T: Display,
{
    async fn create(&self, entity: &T) -> Result<T, anyhow::Error> {
        Self::create_with_executor(entity, &self.pool).await
    }

    async fn create_many(&self, entities: &[T]) -> Result<Vec<T>, anyhow::Error> {
        Self::create_many_with_executor(entities, &self.pool).await
    }

    /// Bulk UPDATE matching `create_many`'s shape. Each entity's `to_params`
    /// drives one UPDATE WHERE id = $1; all run in a single transaction so
    /// the operation is atomic. Slower than a single multi-row VALUES UPDATE
    /// but type-flexible across the heterogeneous column sets we have.
    async fn update_many(&self, entities: &[T]) -> Result<Vec<T>, anyhow::Error> {
        if entities.is_empty() {
            return Ok(vec![]);
        }
        let mut tx = self.pool.begin().await?;
        for entity in entities {
            let (columns, values) = entity.to_params()?;
            let query_str = Self::build_update_query(&columns);
            let mut query = sqlx::query(&query_str);
            for value in &values {
                query = Self::bind_value(query, value)?;
            }
            query.execute(&mut *tx).await?;
        }
        tx.commit().await?;
        tracing::trace!("Bulk updated {} {}s", entities.len(), T::table_name());
        Ok(entities.to_vec())
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<T>, anyhow::Error> {
        let id_filter = StorableFilter::<T>::new_from_entity_id(id);
        // The primary key, so `Multiple` cannot happen — and if it somehow does, the database is
        // in a state worth an error rather than a coin flip.
        self.get_unique(id_filter).await?.at_most_one()
    }

    async fn get_unique(&self, filter: StorableFilter<T>) -> Result<Unique<T>, anyhow::Error> {
        let join_clause = filter.to_join_clause();
        let select = if filter.has_joins() {
            format!("{}.*", T::table_name())
        } else {
            "*".to_string()
        };

        // `LIMIT 2` is the whole mechanism: enough to prove a second row exists, never more than
        // needed to say so. It also makes this strictly cheaper than the unbounded scan it
        // replaces, and removes any need for an `ORDER BY` — the only row it ever returns is one
        // it has already proven unique, so there is nothing left for an ordering to pick between.
        let query_str = format!(
            "SELECT {} FROM {} {} {} LIMIT 2",
            select,
            T::table_name(),
            join_clause,
            filter.to_where_clause()
        );

        let mut query = sqlx::query(&query_str);

        for value in filter.values() {
            query = Self::bind_value(query, value)?;
        }

        let mut rows = query.fetch_all(&self.pool).await?;

        if rows.len() > 1 {
            return Ok(Unique::Multiple);
        }
        match rows.pop() {
            Some(row) => Ok(Unique::One(T::from_row(&row)?)),
            None => Ok(Unique::None),
        }
    }

    async fn get_all(&self, filter: StorableFilter<T>) -> Result<Vec<T>, anyhow::Error> {
        self.get_all_ordered(filter, "created_at ASC").await
    }

    async fn get_all_ordered(
        &self,
        filter: StorableFilter<T>,
        order_by: &str,
    ) -> Result<Vec<T>, anyhow::Error> {
        let pagination_clause = filter.to_pagination_clause();
        let join_clause = filter.to_join_clause();

        // Use table-qualified SELECT when JOINing to avoid column conflicts
        let select = if filter.has_joins() {
            format!("{}.*", T::table_name())
        } else {
            "*".to_string()
        };

        let query_str = format!(
            "SELECT {} FROM {} {} {} ORDER BY {} {}",
            select,
            T::table_name(),
            join_clause,
            filter.to_where_clause(),
            order_by,
            pagination_clause
        );

        let mut query = sqlx::query(&query_str);
        for value in filter.values() {
            query = Self::bind_value(query, value)?;
        }

        let rows = query.fetch_all(&self.pool).await?;
        rows.into_iter().map(|r| T::from_row(&r)).collect()
    }

    async fn count(&self, filter: StorableFilter<T>) -> Result<u64, anyhow::Error> {
        // Include JOIN so filters on joined tables count correctly.
        let count_query_str = format!(
            "SELECT COUNT(*) FROM {} {} {}",
            T::table_name(),
            filter.to_join_clause(),
            filter.to_where_clause()
        );
        let mut count_query = sqlx::query(&count_query_str);
        for value in filter.values() {
            count_query = Self::bind_value(count_query, value)?;
        }
        let count_row = count_query.fetch_one(&self.pool).await?;
        let total_count: i64 = sqlx::Row::get(&count_row, 0);
        Ok(total_count as u64)
    }

    async fn count_distinct_by_group(
        &self,
        filter: StorableFilter<T>,
        group_sql: &str,
        distinct_sql: &str,
    ) -> Result<Vec<(Option<String>, u64)>, anyhow::Error> {
        let query_str = format!(
            "SELECT ({expr})::text, COUNT(DISTINCT {distinct}) FROM {table} {joins} {filter} \
             GROUP BY {expr} ORDER BY {expr} ASC",
            expr = group_sql,
            distinct = distinct_sql,
            table = T::table_name(),
            joins = filter.to_join_clause(),
            filter = filter.to_where_clause(),
        );

        let mut query = sqlx::query(&query_str);
        for value in filter.values() {
            query = Self::bind_value(query, value)?;
        }

        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let value: Option<String> = sqlx::Row::get(&row, 0);
                let count: i64 = sqlx::Row::get(&row, 1);
                (value, count as u64)
            })
            .collect())
    }

    async fn count_by_group(
        &self,
        filter: StorableFilter<T>,
        group_sql: &str,
    ) -> Result<Vec<(Option<String>, u64)>, anyhow::Error> {
        // The filter's limit/offset are deliberately not applied: these are the
        // totals for the whole result set, which is the whole point — the
        // caller already knows how much of each group is on the page.
        //
        // GROUP BY / ORDER BY use the raw expression while only the SELECT
        // casts to text, so the group order matches the list query's ORDER BY
        // exactly rather than whatever the text rendering would collate to.
        let query_str = format!(
            "SELECT ({expr})::text, COUNT(*) FROM {table} {joins} {filter} \
             GROUP BY {expr} ORDER BY {expr} ASC",
            expr = group_sql,
            table = T::table_name(),
            joins = filter.to_join_clause(),
            filter = filter.to_where_clause(),
        );

        let mut query = sqlx::query(&query_str);
        for value in filter.values() {
            query = Self::bind_value(query, value)?;
        }

        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let value: Option<String> = sqlx::Row::get(&row, 0);
                let count: i64 = sqlx::Row::get(&row, 1);
                (value, count as u64)
            })
            .collect())
    }

    async fn get_paginated(
        &self,
        filter: StorableFilter<T>,
        order_by: &str,
    ) -> Result<PaginatedResult<T>, anyhow::Error> {
        let join_clause = filter.to_join_clause();

        // First, get the total count (without limit/offset)
        // Include JOIN in count query to ensure correct count when filtering on joined tables
        let count_query_str = format!(
            "SELECT COUNT(*) FROM {} {} {}",
            T::table_name(),
            join_clause,
            filter.to_where_clause()
        );

        let mut count_query = sqlx::query(&count_query_str);
        for value in filter.values() {
            count_query = Self::bind_value(count_query, value)?;
        }

        let count_row = count_query.fetch_one(&self.pool).await?;
        let total_count: i64 = sqlx::Row::get(&count_row, 0);
        let total_count = total_count as u64;

        // Then get the paginated results
        let pagination_clause = filter.to_pagination_clause();

        // Use table-qualified SELECT when JOINing to avoid column conflicts
        let select = if filter.has_joins() {
            format!("{}.*", T::table_name())
        } else {
            "*".to_string()
        };

        let query_str = format!(
            "SELECT {} FROM {} {} {} ORDER BY {} {}",
            select,
            T::table_name(),
            join_clause,
            filter.to_where_clause(),
            order_by,
            pagination_clause
        );

        let mut query = sqlx::query(&query_str);
        for value in filter.values() {
            query = Self::bind_value(query, value)?;
        }

        let rows = query.fetch_all(&self.pool).await?;
        let items: Vec<T> = rows
            .into_iter()
            .map(|r| T::from_row(&r))
            .collect::<Result<_, _>>()?;

        Ok(PaginatedResult { items, total_count })
    }

    async fn update(&self, entity: &mut T) -> Result<T, anyhow::Error> {
        // Note: set_updated_at is called by the service layer for Entity types.
        // The storage layer just persists the entity as-is.
        let (columns, values) = entity.to_params()?;
        let query_str = Self::build_update_query(&columns);

        let mut query = sqlx::query(&query_str);
        for value in &values {
            query = Self::bind_value(query, value)?;
        }

        tracing::trace!("Updated {}", entity);

        query.execute(&self.pool).await?;
        Ok(entity.clone())
    }

    async fn delete(&self, id: &Uuid) -> Result<(), anyhow::Error> {
        let query_str = format!("DELETE FROM {} WHERE id = $1", T::table_name());

        sqlx::query(&query_str).bind(id).execute(&self.pool).await?;

        tracing::trace!("Deleted {} with id: {}", T::table_name(), id);

        Ok(())
    }

    async fn delete_many(&self, ids: &[Uuid]) -> Result<usize, anyhow::Error> {
        if ids.is_empty() {
            return Ok(0);
        }

        let query_str = format!("DELETE FROM {} WHERE id = ANY($1)", T::table_name());

        let result = sqlx::query(&query_str)
            .bind(ids)
            .execute(&self.pool)
            .await?;

        let deleted_count = result.rows_affected() as usize;

        tracing::trace!(
            "Bulk deleted {} {}s (requested: {}, deleted: {})",
            deleted_count,
            T::table_name(),
            ids.len(),
            deleted_count
        );

        Ok(deleted_count)
    }

    async fn delete_by_filter(&self, filter: StorableFilter<T>) -> Result<usize, anyhow::Error> {
        Self::delete_by_filter_with_executor(filter, &self.pool).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The codes that reach here when a device sends something PostgreSQL cannot store. Each one
    /// used to be classified as systemic, which abandoned the whole entity rather than the row —
    /// on GH #668 that turned one NUL byte on one switch port into a lost host.
    #[test]
    fn data_exceptions_are_recognised_by_class() {
        for code in [
            "22021", // character_not_in_repertoire — a NUL in a text column
            "22P05", // untranslatable_character — the same escape in jsonb
            "22001", // string_data_right_truncation
            "22003", // numeric_value_out_of_range
            "22007", // invalid_datetime_format
            "22P02", // invalid_text_representation
        ] {
            assert!(
                is_data_exception_code(Some(code)),
                "{code} should be a data exception"
            );
        }
    }

    /// Anything genuinely about the database, the connection or the schema must keep aborting the
    /// caller — degrading these to "skip the row" would hide real faults.
    #[test]
    fn other_classes_are_not_data_exceptions() {
        for code in [
            "23505", // unique_violation — handled separately, and before this check
            "23503", // foreign_key_violation
            "40001", // serialization_failure
            "53300", // too_many_connections
            "42P01", // undefined_table
            "08006", // connection_failure
        ] {
            assert!(
                !is_data_exception_code(Some(code)),
                "{code} must stay systemic"
            );
        }
        assert!(!is_data_exception_code(None));
    }

    /// SQLSTATE is fixed-width, so a bare prefix test would over-match.
    #[test]
    fn a_longer_code_beginning_22_is_not_matched() {
        assert!(!is_data_exception_code(Some("22021X")));
        assert!(!is_data_exception_code(Some("22")));
    }
}
