//! Daemon ↔ interfaced-subnet junction table types.
//!
//! Models the `daemon_interfaced_subnets` junction table, stored through
//! `GenericPostgresStorage`. Each row belongs to its daemon and its subnet. Replaces the referential-
//! integrity-free `capabilities.interfaced_subnet_ids` JSONB blob: a real FK to
//! `subnets(id) ON DELETE CASCADE` means deleting a subnet removes its junction
//! rows instead of leaving a dangling id. Cardinality is many-to-many (daemons
//! sharing a CIDR match the same subnet row), so it's a composite-key junction.

use anyhow::Result;
use sqlx::{Row, postgres::PgRow};
use std::fmt::Display;
use uuid::Uuid;

use crate::server::shared::storage::traits::{SqlValue, Storable};

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

    fn relations() -> &'static [&'static crate::server::shared::storage::relation::Relation<Self>] {
        use crate::server::shared::relations as R;
        &[&R::INTERFACED_SUBNET_DAEMON, &R::INTERFACED_SUBNET_SUBNET]
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
