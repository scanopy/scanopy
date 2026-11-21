//! A scan's subnet targets, as `discovery_subnets` junction rows.
//!
//! A scan names the subnets it covers. Kept as ids inside `discovery_type`, a deleted subnet left
//! a dangling id the daemon silently skipped, so a scan whose every target was deleted scanned
//! nothing and completed clean. The junction's foreign key to `subnets(id) ON DELETE CASCADE`
//! drops the target with its subnet. A scan with no target rows reads as targeting none, which
//! covers the subnets its daemon has an interface on.

use anyhow::Result;
use sqlx::{Row, postgres::PgRow};
use std::fmt::Display;
use uuid::Uuid;

use crate::server::shared::storage::traits::{SqlValue, Storable};

/// A scan's target subnet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiscoverySubnet {
    pub discovery_id: Uuid,
    pub subnet_id: Uuid,
}

impl Display for DiscoverySubnet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DiscoverySubnet(discovery={}, subnet={})",
            self.discovery_id, self.subnet_id
        )
    }
}

impl Storable for DiscoverySubnet {
    const HAS_SCD2: bool = false;
    type BaseData = (Uuid, Uuid);

    fn table_name() -> &'static str {
        "discovery_subnets"
    }

    fn relations() -> &'static [&'static crate::server::shared::storage::relation::Relation<Self>] {
        use crate::server::shared::relations as R;
        &[&R::DISCOVERY_SUBNET_DISCOVERY, &R::DISCOVERY_SUBNET_SUBNET]
    }

    fn new(base: Self::BaseData) -> Self {
        Self {
            discovery_id: base.0,
            subnet_id: base.1,
        }
    }

    fn get_base(&self) -> Self::BaseData {
        (self.discovery_id, self.subnet_id)
    }

    fn to_params(&self) -> Result<(Vec<&'static str>, Vec<SqlValue>)> {
        Ok((
            vec!["discovery_id", "subnet_id"],
            vec![
                SqlValue::Uuid(self.discovery_id),
                SqlValue::Uuid(self.subnet_id),
            ],
        ))
    }

    fn from_row(row: &PgRow) -> Result<Self> {
        Ok(Self {
            discovery_id: row.get("discovery_id"),
            subnet_id: row.get("subnet_id"),
        })
    }
}
