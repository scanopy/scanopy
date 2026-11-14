use sqlx::{Row, postgres::PgRow};

use crate::{
    daemon::discovery::types::warnings::warning_from_value,
    server::{
        discovery_warnings::r#impl::base::{DiscoveryWarningRecord, DiscoveryWarningRecordBase},
        shared::storage::traits::{SqlValue, Storable},
    },
};

impl Storable for DiscoveryWarningRecord {
    const HAS_SCD2: bool = false;
    type BaseData = DiscoveryWarningRecordBase;

    fn table_name() -> &'static str {
        "discovery_warnings"
    }

    fn relations() -> &'static [&'static crate::server::shared::storage::relation::Relation<Self>] {
        use crate::server::shared::relations as R;
        &[&R::DISCOVERY_WARNING_DISCOVERY]
    }

    fn new(base: Self::BaseData) -> Self {
        DiscoveryWarningRecord::new(base)
    }

    fn get_base(&self) -> Self::BaseData {
        self.base.clone()
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>), anyhow::Error> {
        Ok((
            vec![
                "id",
                "discovery_id",
                "site_id",
                "code",
                "credential_id",
                "host_id",
                "remote_host_id",
                "subnet_id",
                "details",
                "created_at",
            ],
            vec![
                SqlValue::Uuid(self.id),
                SqlValue::Uuid(self.base.discovery_id),
                SqlValue::Uuid(self.base.site_id),
                SqlValue::DiscoveryWarningCode(self.base.details.code()),
                SqlValue::OptionalUuid(self.base.credential_id),
                SqlValue::OptionalUuid(self.base.host_id),
                SqlValue::OptionalUuid(self.base.remote_host_id),
                SqlValue::OptionalUuid(self.base.subnet_id),
                SqlValue::DiscoveryWarningDetails(self.base.details.clone()),
                SqlValue::Timestamp(self.created_at),
            ],
        ))
    }

    /// `code` is not read back: it is derived from `details` on every write, for queries that
    /// count or group by it, and the details are what the warning is rebuilt from.
    fn from_row(row: &PgRow) -> Result<Self, anyhow::Error> {
        let details: serde_json::Value = row.try_get("details")?;
        Ok(DiscoveryWarningRecord {
            id: row.try_get("id")?,
            created_at: row.try_get("created_at")?,
            base: DiscoveryWarningRecordBase {
                discovery_id: row.try_get("discovery_id")?,
                site_id: row.try_get("site_id")?,
                credential_id: row.try_get("credential_id")?,
                host_id: row.try_get("host_id")?,
                remote_host_id: row.try_get("remote_host_id")?,
                subnet_id: row.try_get("subnet_id")?,
                details: warning_from_value(details),
            },
        })
    }
}
