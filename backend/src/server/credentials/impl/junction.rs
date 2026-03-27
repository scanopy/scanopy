//! Credential junction table types and storage.
//!
//! Models the `network_credentials` and `host_credentials` junction tables
//! using `Storable` + `GenericPostgresStorage` instead of raw SQL.

use anyhow::Result;
use sqlx::{PgPool, Row, postgres::PgRow};
use std::collections::HashMap;
use std::fmt::Display;
use uuid::Uuid;

use crate::server::{
    credentials::r#impl::types::CredentialAssignment,
    shared::storage::{
        filter::StorableFilter,
        generic::GenericPostgresStorage,
        traits::{SqlValue, Storable, Storage},
    },
};

// =============================================================================
// NetworkCredential (Junction Table)
// =============================================================================

/// A junction record linking a network to a credential.
#[derive(Debug, Clone, Default)]
pub struct NetworkCredential {
    pub network_id: Uuid,
    pub credential_id: Uuid,
}

impl Display for NetworkCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NetworkCredential(network={}, credential={})",
            self.network_id, self.credential_id
        )
    }
}

impl Storable for NetworkCredential {
    type BaseData = (Uuid, Uuid);

    fn table_name() -> &'static str {
        "network_credentials"
    }

    fn new(base: Self::BaseData) -> Self {
        Self {
            network_id: base.0,
            credential_id: base.1,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        (self.network_id, self.credential_id)
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["network_id", "credential_id"],
            vec![
                SqlValue::Uuid(self.network_id),
                SqlValue::Uuid(self.credential_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(Self {
            network_id: row.get("network_id"),
            credential_id: row.get("credential_id"),
        })
    }
}

// =============================================================================
// HostCredential (Junction Table)
// =============================================================================

/// A junction record linking a host to a credential, optionally scoped to interfaces.
#[derive(Debug, Clone, Default)]
pub struct HostCredential {
    pub host_id: Uuid,
    pub credential_id: Uuid,
    pub interface_ids: Option<Vec<Uuid>>,
}

impl Display for HostCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HostCredential(host={}, credential={})",
            self.host_id, self.credential_id
        )
    }
}

impl Storable for HostCredential {
    type BaseData = (Uuid, Uuid, Option<Vec<Uuid>>);

    fn table_name() -> &'static str {
        "host_credentials"
    }

    fn new(base: Self::BaseData) -> Self {
        Self {
            host_id: base.0,
            credential_id: base.1,
            interface_ids: base.2,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        (self.host_id, self.credential_id, self.interface_ids.clone())
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["host_id", "credential_id", "interface_ids"],
            vec![
                SqlValue::Uuid(self.host_id),
                SqlValue::Uuid(self.credential_id),
                SqlValue::OptionalUuidVec(self.interface_ids.clone()),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(Self {
            host_id: row.get("host_id"),
            credential_id: row.get("credential_id"),
            interface_ids: row.get("interface_ids"),
        })
    }
}

// =============================================================================
// NetworkCredentialStorage
// =============================================================================

/// Storage operations for `network_credentials` junction table.
pub struct NetworkCredentialStorage {
    storage: GenericPostgresStorage<NetworkCredential>,
}

impl NetworkCredentialStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get credential IDs for a network.
    pub async fn get_credential_ids_for_network(&self, network_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter =
            StorableFilter::<NetworkCredential>::new_from_uuid_column("network_id", network_id);
        let records = self
            .storage
            .get_all_ordered(filter, "credential_id ASC")
            .await?;
        Ok(records.into_iter().map(|r| r.credential_id).collect())
    }

    /// Get credential IDs for multiple networks (batch).
    pub async fn get_credential_ids_for_networks(
        &self,
        network_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if network_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let filter =
            StorableFilter::<NetworkCredential>::new_from_uuids_column("network_id", network_ids);
        let records = self
            .storage
            .get_all_ordered(filter, "network_id ASC")
            .await?;

        let mut map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for record in records {
            map.entry(record.network_id)
                .or_default()
                .push(record.credential_id);
        }
        Ok(map)
    }

    /// Replace all credentials for a network (atomic).
    pub async fn save_for_network(&self, network_id: &Uuid, credential_ids: &[Uuid]) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;

        let filter =
            StorableFilter::<NetworkCredential>::new_from_uuid_column("network_id", network_id);
        tx.delete_by_filter(filter).await?;

        for cred_id in credential_ids {
            let record = NetworkCredential {
                network_id: *network_id,
                credential_id: *cred_id,
            };
            tx.create(&record).await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

// =============================================================================
// HostCredentialStorage
// =============================================================================

/// Storage operations for `host_credentials` junction table.
pub struct HostCredentialStorage {
    storage: GenericPostgresStorage<HostCredential>,
}

impl HostCredentialStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get credential assignments for a host.
    pub async fn get_assignments_for_host(
        &self,
        host_id: &Uuid,
    ) -> Result<Vec<CredentialAssignment>> {
        let filter = StorableFilter::<HostCredential>::new_from_uuid_column("host_id", host_id);
        let records = self
            .storage
            .get_all_ordered(filter, "credential_id ASC")
            .await?;
        Ok(records
            .into_iter()
            .map(|r| CredentialAssignment {
                credential_id: r.credential_id,
                interface_ids: r.interface_ids,
            })
            .collect())
    }

    /// Get credential assignments for multiple hosts (batch).
    pub async fn get_assignments_for_hosts(
        &self,
        host_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<CredentialAssignment>>> {
        if host_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let filter = StorableFilter::<HostCredential>::new_from_uuids_column("host_id", host_ids);
        let records = self.storage.get_all_ordered(filter, "host_id ASC").await?;

        let mut map: HashMap<Uuid, Vec<CredentialAssignment>> = HashMap::new();
        for record in records {
            map.entry(record.host_id)
                .or_default()
                .push(CredentialAssignment {
                    credential_id: record.credential_id,
                    interface_ids: record.interface_ids,
                });
        }
        Ok(map)
    }

    /// Replace all credential assignments for a host (atomic).
    pub async fn save_for_host(
        &self,
        host_id: &Uuid,
        assignments: &[CredentialAssignment],
    ) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;

        let filter = StorableFilter::<HostCredential>::new_from_uuid_column("host_id", host_id);
        tx.delete_by_filter(filter).await?;

        for assignment in assignments {
            let record = HostCredential {
                host_id: *host_id,
                credential_id: assignment.credential_id,
                interface_ids: assignment.interface_ids.clone(),
            };
            tx.create(&record).await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
