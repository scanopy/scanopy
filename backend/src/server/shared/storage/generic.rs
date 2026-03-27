use crate::server::shared::{
    storage::{
        filter::StorableFilter,
        traits::{PaginatedResult, SqlValue, Storable, Storage},
    },
    types::api::ValidationError,
};
use async_trait::async_trait;
use ipnetwork::IpNetwork;
use sqlx::{Executor, PgPool, Postgres, postgres::PgArguments};
use std::{fmt::Display, marker::PhantomData};
use uuid::Uuid;

// Re-export for convenience
pub use sqlx::postgres::PgConnection;

pub struct GenericPostgresStorage<T: Storable> {
    pool: PgPool,
    _phantom: PhantomData<T>,
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

    /// Convert a unique constraint violation into a user-friendly message
    fn friendly_unique_violation_message(constraint: Option<&str>) -> String {
        match constraint {
            // ports(host_id, port_number, protocol)
            Some(c) if c.contains("ports") => {
                "A port with this number and protocol already exists on this host".to_string()
            }
            // interfaces(host_id, subnet_id, ip_address)
            Some(c) if c.contains("interfaces") => {
                "An interface with this IP address already exists on this host".to_string()
            }
            // tags(organization_id, name)
            Some(c) if c.contains("tags") => "A tag with this name already exists".to_string(),
            // group_bindings(group_id, binding_id)
            Some(c) if c.contains("group_bindings") => {
                "This binding already exists in the group".to_string()
            }
            // user_network_access(user_id, network_id)
            Some(c) if c.contains("user_network_access") => {
                "This user already has access to this network".to_string()
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

    /// Bind SqlValue to query
    fn bind_value<'q>(
        query: sqlx::query::Query<'q, Postgres, PgArguments>,
        value: &'q SqlValue,
    ) -> Result<sqlx::query::Query<'q, Postgres, PgArguments>, anyhow::Error> {
        let value = match value {
            SqlValue::Uuid(v) => query.bind(v),
            SqlValue::OptionalUuid(v) => query.bind(v),
            SqlValue::String(v) => query.bind(v),
            SqlValue::U16(v) => query.bind(Into::<i32>::into(*v)),
            SqlValue::I32(v) => query.bind(v),
            SqlValue::OptionalI64(v) => query.bind(v),
            SqlValue::Bool(v) => query.bind(v),
            SqlValue::Timestamp(v) => query.bind(v),
            SqlValue::OptionTimestamp(v) => query.bind(v),
            SqlValue::UuidArray(v) => query.bind(v.clone()),
            SqlValue::OptionalString(v) => query.bind(v),
            SqlValue::EntitySource(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::IpCidr(v) => query.bind(serde_json::to_string(v)?),
            SqlValue::ServiceDefinition(v) => query.bind(serde_json::to_string(v)?),
            SqlValue::OptionalServiceVirtualization(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::Interfaces(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::Ports(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::Bindings(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::OptionalHostVirtualization(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::DaemonCapabilities(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::IpAddr(v) => {
                // Convert IpAddr to IpNetwork for proper INET binding
                let network = IpNetwork::from(*v);
                query.bind(network)
            }
            SqlValue::OptionalIpAddr(v) => {
                // Convert Option<IpAddr> to Option<IpNetwork> for proper INET binding
                let network = v.map(IpNetwork::from);
                query.bind(network)
            }
            SqlValue::RunType(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::DiscoveryType(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::Email(v) => query.bind(v.as_str()),
            SqlValue::UserOrgPermissions(v) => query.bind(v.as_str()),
            SqlValue::DaemonMode(v) => query.bind(serde_json::to_string(v)?),
            SqlValue::OptionBillingPlan(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::OptionBillingPlanStatus(v) => query.bind(serde_json::to_string(v)?),
            SqlValue::EdgeStyle(v) => query.bind(v.to_string()),
            SqlValue::Nodes(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::Edges(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::TopologyOptions(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::Hosts(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::Subnets(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::Services(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::Groups(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::IfEntries(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::Tags(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::PlanLimitNotifications(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::OnboardingOperation(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::StringArray(v) => query.bind(v.clone()),
            SqlValue::OptionalStringArray(v) => query.bind(v.clone()),
            SqlValue::OptionalLldpChassisId(v) => {
                query.bind(v.as_ref().map(|c| serde_json::to_value(c).unwrap()))
            }
            SqlValue::OptionalLldpPortId(v) => {
                query.bind(v.as_ref().map(|p| serde_json::to_value(p).unwrap()))
            }
            SqlValue::OptionalFdbMacs(v) => {
                query.bind(v.as_ref().map(|m| serde_json::to_value(m).unwrap()))
            }
            SqlValue::ShareOptions(v) => query.bind(serde_json::to_value(v)?),
            SqlValue::CredentialType(v) => query.bind(serde_json::to_value(
                crate::server::credentials::r#impl::types::StorageCredentialType(v),
            )?),
            SqlValue::MacAddress(v) => {
                // sqlx mac_address feature supports MacAddress directly
                query.bind(*v)
            }
            SqlValue::OptionalMacAddress(v) => {
                // sqlx mac_address feature supports MacAddress directly
                query.bind(*v)
            }
            SqlValue::OptionalIpAddrArray(v) => {
                let networks: Option<Vec<IpNetwork>> = v
                    .as_ref()
                    .map(|ips| ips.iter().map(|ip| IpNetwork::from(*ip)).collect());
                query.bind(networks)
            }
            SqlValue::EntityDiscriminant(v) => {
                // Serialize to JSON string to match how it's stored/deserialized
                query.bind(serde_json::to_string(v)?)
            }
            SqlValue::OptionalUuidVec(v) => query.bind(v.clone()),
        };

        Ok(value)
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

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<T>, anyhow::Error> {
        let id_filter = StorableFilter::<T>::new_from_entity_id(id);
        self.get_one(id_filter).await
    }

    async fn get_one(&self, filter: StorableFilter<T>) -> Result<Option<T>, anyhow::Error> {
        let query_str = format!(
            "SELECT * FROM {} {}",
            T::table_name(),
            filter.to_where_clause()
        );

        let mut query = sqlx::query(&query_str);

        for value in filter.values() {
            query = Self::bind_value(query, value)?;
        }

        let row = query.fetch_optional(&self.pool).await?;

        let result = row.map(|r| T::from_row(&r)).transpose()?;

        Ok(result)
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
