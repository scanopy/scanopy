//! Credential junction table types and storage.
//!
//! Models the `site_credentials` and `host_credentials` junction tables
//! using `Storable` + `GenericPostgresStorage` instead of raw SQL.

use anyhow::Result;
use sqlx::{PgPool, Row, postgres::PgRow};
use std::collections::HashMap;
use std::fmt::Display;
use uuid::Uuid;

use crate::server::{
    credentials::r#impl::types::{CredentialAssignment, CredentialHostAssignment},
    shared::{
        entities::EntityDiscriminants,
        storage::{
            filter::StorableFilter,
            generic::GenericPostgresStorage,
            lock::{DEFAULT_LOCK_TIMEOUT, LockKey},
            traits::{SqlValue, Storable, Storage},
        },
    },
};

// =============================================================================
// SiteCredential (Junction Table)
// =============================================================================

/// A junction record linking a site to a credential.
#[derive(Debug, Clone, Default)]
pub struct SiteCredential {
    pub site_id: Uuid,
    pub credential_id: Uuid,
}

impl Display for SiteCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SiteCredential(site={}, credential={})",
            self.site_id, self.credential_id
        )
    }
}

impl Storable for SiteCredential {
    const HAS_SCD2: bool = false;
    type BaseData = (Uuid, Uuid);

    fn table_name() -> &'static str {
        "site_credentials"
    }

    fn new(base: Self::BaseData) -> Self {
        Self {
            site_id: base.0,
            credential_id: base.1,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        (self.site_id, self.credential_id)
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["site_id", "credential_id"],
            vec![
                SqlValue::Uuid(self.site_id),
                SqlValue::Uuid(self.credential_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(Self {
            site_id: row.get("site_id"),
            credential_id: row.get("credential_id"),
        })
    }
}

// =============================================================================
// HostCredential (Junction Table)
// =============================================================================

/// A junction record linking a host to a credential, optionally scoped to ip_addresses.
#[derive(Debug, Clone, Default)]
pub struct HostCredential {
    pub host_id: Uuid,
    pub credential_id: Uuid,
    pub ip_address_ids: Option<Vec<Uuid>>,
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
    const HAS_SCD2: bool = false;
    type BaseData = (Uuid, Uuid, Option<Vec<Uuid>>);

    fn table_name() -> &'static str {
        "host_credentials"
    }

    fn new(base: Self::BaseData) -> Self {
        Self {
            host_id: base.0,
            credential_id: base.1,
            ip_address_ids: base.2,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        (
            self.host_id,
            self.credential_id,
            self.ip_address_ids.clone(),
        )
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["host_id", "credential_id", "ip_address_ids"],
            vec![
                SqlValue::Uuid(self.host_id),
                SqlValue::Uuid(self.credential_id),
                SqlValue::OptionalUuidVec(self.ip_address_ids.clone()),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(Self {
            host_id: row.get("host_id"),
            credential_id: row.get("credential_id"),
            ip_address_ids: row.get("ip_address_ids"),
        })
    }
}

// =============================================================================
// SiteCredentialStorage
// =============================================================================

/// Storage operations for `site_credentials` junction table.
pub struct SiteCredentialStorage {
    storage: GenericPostgresStorage<SiteCredential>,
}

impl SiteCredentialStorage {
    pub fn new(pool: PgPool) -> Self {
        Self {
            storage: GenericPostgresStorage::new(pool),
        }
    }

    /// Get credential IDs for a site.
    pub async fn get_credential_ids_for_site(&self, site_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter = StorableFilter::<SiteCredential>::new_from_uuid_column("site_id", site_id);
        let records = self
            .storage
            .get_all_ordered(filter, "credential_id ASC")
            .await?;
        Ok(records.into_iter().map(|r| r.credential_id).collect())
    }

    /// Get credential IDs for multiple sites (batch).
    pub async fn get_credential_ids_for_sites(
        &self,
        site_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if site_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let filter = StorableFilter::<SiteCredential>::new_from_uuids_column("site_id", site_ids);
        let records = self.storage.get_all_ordered(filter, "site_id ASC").await?;

        let mut map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for record in records {
            map.entry(record.site_id)
                .or_default()
                .push(record.credential_id);
        }
        Ok(map)
    }

    /// Replace all credentials for a site (atomic).
    /// Bulk-insert site↔credential rows in a single INSERT, with no
    /// per-site lock or delete-first pass. For seed paths (demo populate) on
    /// a freshly reset org where there are no existing rows to replace.
    pub async fn create_many(&self, records: &[SiteCredential]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        self.storage.create_many(records).await?;
        Ok(())
    }

    pub async fn save_for_site(&self, site_id: &Uuid, credential_ids: &[Uuid]) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;
        // Serialize concurrent delete-all + re-insert syncs for one site.
        tx.lock(
            LockKey::JunctionSync {
                parent: EntityDiscriminants::Site,
                parent_id: *site_id,
            },
            DEFAULT_LOCK_TIMEOUT,
        )
        .await?;

        let filter = StorableFilter::<SiteCredential>::new_from_uuid_column("site_id", site_id);
        tx.delete_by_filter(filter).await?;

        for cred_id in credential_ids {
            let record = SiteCredential {
                site_id: *site_id,
                credential_id: *cred_id,
            };
            tx.create(&record).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Get the site IDs a credential is assigned to (reverse lookup).
    pub async fn get_site_ids_for_credential(&self, credential_id: &Uuid) -> Result<Vec<Uuid>> {
        let filter =
            StorableFilter::<SiteCredential>::new_from_uuid_column("credential_id", credential_id);
        let records = self.storage.get_all_ordered(filter, "site_id ASC").await?;
        Ok(records.into_iter().map(|r| r.site_id).collect())
    }

    /// Get the site IDs for multiple credentials (batch, reverse lookup).
    pub async fn get_site_ids_for_credentials(
        &self,
        credential_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if credential_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let filter = StorableFilter::<SiteCredential>::new_from_uuids_column(
            "credential_id",
            credential_ids,
        );
        let records = self
            .storage
            .get_all_ordered(filter, "credential_id ASC")
            .await?;

        let mut map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for record in records {
            map.entry(record.credential_id)
                .or_default()
                .push(record.site_id);
        }
        Ok(map)
    }

    /// Replace the full set of sites a credential is assigned to (atomic).
    /// Only touches rows for this credential, so other credentials on the same
    /// sites are left untouched.
    pub async fn save_sites_for_credential(
        &self,
        credential_id: &Uuid,
        site_ids: &[Uuid],
    ) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;
        // Serialize concurrent delete-all + re-insert syncs for one credential.
        tx.lock(
            LockKey::JunctionSync {
                parent: EntityDiscriminants::Credential,
                parent_id: *credential_id,
            },
            DEFAULT_LOCK_TIMEOUT,
        )
        .await?;

        let filter =
            StorableFilter::<SiteCredential>::new_from_uuid_column("credential_id", credential_id);
        tx.delete_by_filter(filter).await?;

        for site_id in site_ids {
            let record = SiteCredential {
                site_id: *site_id,
                credential_id: *credential_id,
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
                ip_address_ids: r.ip_address_ids,
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
                    ip_address_ids: record.ip_address_ids,
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
        // Serialize concurrent delete-all + re-insert syncs for one host.
        tx.lock(
            LockKey::JunctionSync {
                parent: EntityDiscriminants::Host,
                parent_id: *host_id,
            },
            DEFAULT_LOCK_TIMEOUT,
        )
        .await?;

        let filter = StorableFilter::<HostCredential>::new_from_uuid_column("host_id", host_id);
        tx.delete_by_filter(filter).await?;

        for assignment in assignments {
            let record = HostCredential {
                host_id: *host_id,
                credential_id: assignment.credential_id,
                ip_address_ids: assignment.ip_address_ids.clone(),
            };
            tx.create(&record).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Get the host assignments for a credential (reverse lookup).
    pub async fn get_host_assignments_for_credential(
        &self,
        credential_id: &Uuid,
    ) -> Result<Vec<CredentialHostAssignment>> {
        let filter =
            StorableFilter::<HostCredential>::new_from_uuid_column("credential_id", credential_id);
        let records = self.storage.get_all_ordered(filter, "host_id ASC").await?;
        Ok(records
            .into_iter()
            .map(|r| CredentialHostAssignment {
                host_id: r.host_id,
                ip_address_ids: r.ip_address_ids,
            })
            .collect())
    }

    /// Get the host assignments for multiple credentials (batch, reverse lookup).
    pub async fn get_host_assignments_for_credentials(
        &self,
        credential_ids: &[Uuid],
    ) -> Result<HashMap<Uuid, Vec<CredentialHostAssignment>>> {
        if credential_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let filter = StorableFilter::<HostCredential>::new_from_uuids_column(
            "credential_id",
            credential_ids,
        );
        let records = self
            .storage
            .get_all_ordered(filter, "credential_id ASC")
            .await?;

        let mut map: HashMap<Uuid, Vec<CredentialHostAssignment>> = HashMap::new();
        for record in records {
            map.entry(record.credential_id)
                .or_default()
                .push(CredentialHostAssignment {
                    host_id: record.host_id,
                    ip_address_ids: record.ip_address_ids,
                });
        }
        Ok(map)
    }

    /// Replace the full set of host assignments for a credential (atomic).
    /// Only touches rows for this credential, so other credentials on the same
    /// hosts are left untouched.
    pub async fn save_host_assignments_for_credential(
        &self,
        credential_id: &Uuid,
        assignments: &[CredentialHostAssignment],
    ) -> Result<()> {
        let mut tx = self.storage.begin_transaction().await?;
        // Serialize concurrent delete-all + re-insert syncs for one credential.
        tx.lock(
            LockKey::JunctionSync {
                parent: EntityDiscriminants::Credential,
                parent_id: *credential_id,
            },
            DEFAULT_LOCK_TIMEOUT,
        )
        .await?;

        let filter =
            StorableFilter::<HostCredential>::new_from_uuid_column("credential_id", credential_id);
        tx.delete_by_filter(filter).await?;

        for assignment in assignments {
            let record = HostCredential {
                host_id: assignment.host_id,
                credential_id: *credential_id,
                ip_address_ids: assignment.ip_address_ids.clone(),
            };
            tx.create(&record).await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
