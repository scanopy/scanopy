//! Daemon ↔ interfaced-subnet junction table types and storage.
//!
//! Models the `daemon_interfaced_subnets` junction table using `Storable` +
//! `GenericPostgresStorage` instead of raw SQL. Replaces the referential-
//! integrity-free `capabilities.interfaced_subnet_ids` JSONB blob: a real FK to
//! `subnets(id) ON DELETE CASCADE` means deleting a subnet removes its junction
//! rows instead of leaving a dangling id. Cardinality is many-to-many (daemons
//! sharing a CIDR match the same subnet row), so it's a composite-key junction.

use anyhow::Result;
use sqlx::{PgPool, Row, postgres::PgRow};
use std::collections::HashMap;
use std::fmt::Display;
use uuid::Uuid;

use crate::server::shared::entities::EntityDiscriminants;
use crate::server::shared::storage::{
    filter::StorableFilter,
    generic::GenericPostgresStorage,
    lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
    traits::{SqlValue, Storable, Storage},
};

/// A junction record linking a daemon to a subnet it has an interface on.
#[derive(Debug, Clone, Default)]
pub struct DaemonInterfacedSubnet {
    pub daemon_id: Uuid,
    pub subnet_id: Uuid,
}

impl Display for DaemonInterfacedSubnet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DaemonInterfacedSubnet(daemon={}, subnet={})",
            self.daemon_id, self.subnet_id
        )
    }
}

impl Storable for DaemonInterfacedSubnet {
    const HAS_SCD2: bool = false;
    type BaseData = (Uuid, Uuid);

    fn table_name() -> &'static str {
        "daemon_interfaced_subnets"
    }

    fn new(base: Self::BaseData) -> Self {
        Self {
            daemon_id: base.0,
            subnet_id: base.1,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        (self.daemon_id, self.subnet_id)
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["daemon_id", "subnet_id"],
            vec![
                SqlValue::Uuid(self.daemon_id),
                SqlValue::Uuid(self.subnet_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(Self {
            daemon_id: row.get("daemon_id"),
            subnet_id: row.get("subnet_id"),
        })
    }
}

/// Storage operations for the `daemon_interfaced_subnets` junction table.
pub struct DaemonInterfacedSubnetStorage {
    storage: GenericPostgresStorage<DaemonInterfacedSubnet>,
}

impl DaemonInterfacedSubnetStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get the subnet IDs a daemon has interfaces on.
    pub async fn get_subnet_ids_for_daemon(&self, daemon_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter =
            StorableFilter::<DaemonInterfacedSubnet>::new_from_uuid_column("daemon_id", daemon_id);
        let records = self
            .storage
            .get_all_ordered(filter, "subnet_id ASC")
            .await?;
        Ok(records.into_iter().map(|r| r.subnet_id).collect())
    }

    /// Get the subnet IDs for multiple daemons (batch — avoids N+1 on list endpoints).
    pub async fn get_subnet_ids_for_daemons(
        &self,
        daemon_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if daemon_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let filter = StorableFilter::<DaemonInterfacedSubnet>::new_from_uuids_column(
            "daemon_id",
            daemon_ids,
        );
        let records = self
            .storage
            .get_all_ordered(filter, "daemon_id ASC")
            .await?;

        let mut map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for record in records {
            map.entry(record.daemon_id)
                .or_default()
                .push(record.subnet_id);
        }
        Ok(map)
    }

    /// Replace the full set of interfaced subnets for a daemon (atomic).
    ///
    /// Callers must pass only `subnet_id`s that exist in `subnets` — the FK is
    /// enforced. The service layer is responsible for resolving/validating ids
    /// (via `SubnetService`), keeping subnet-existence checks out of this storage
    /// layer (no cross-entity storage access).
    pub async fn save_interfaced_subnets_for_daemon(
        &self,
        daemon_id: &Uuid,
        subnet_ids: &[Uuid],
    ) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;
        // Serialize concurrent delete-all + re-insert syncs for one daemon.
        tx.lock(
            LockKey::JunctionSync {
                parent: EntityDiscriminants::Daemon,
                parent_id: *daemon_id,
            },
            DEFAULT_LOCK_TIMEOUT,
        )
        .await?;

        let filter =
            StorableFilter::<DaemonInterfacedSubnet>::new_from_uuid_column("daemon_id", daemon_id);
        tx.delete_by_filter(filter).await?;

        for subnet_id in subnet_ids {
            let record = DaemonInterfacedSubnet {
                daemon_id: *daemon_id,
                subnet_id: *subnet_id,
            };
            tx.create(&record).await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
